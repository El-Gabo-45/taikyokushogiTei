//! Lock-free bucketed transposition table, with a runtime-configurable size.
//!
//! Layout: `buckets` buckets × `TT_BUCKET_WIDTH` entries; each entry is TWO
//! AtomicU64 slots — hash and payload. Payload is published BEFORE the hash
//! (Release) and re-verified after read (Acquire) so concurrent writers
//! (Lazy SMP) never yield a torn entry: a reader that sees the hash is
//! guaranteed to see at least that data snapshot.
//!
//! ## Sizing
//! The table used to be a hard-coded `1 << 22` buckets (~256 MiB) allocated
//! lazily from a `OnceLock`: a 256 MiB floor for *every* process (unit tests,
//! each self-play worker, small CLI runs) with no way to ask for less. The
//! storage now lives behind an `RwLock<Arc<TtStorage>>`:
//!
//! * probes/stores take a *read* lock for the duration of a single lookup
//!   (a few nanoseconds — nodes cost ~150 µs here, so it is noise), and
//! * [`resize_mb`] takes the write lock, installs a fresh table and bumps the
//!   generation. In-flight readers keep the old `Arc` alive, so resizing is
//!   race-free and can happen between iterations of a running search.
//!
//! The default stays at the historical 256 MiB; [`super::set_hash_mb`] lets a
//! caller pick something else, and because the table persists across searches
//! it can be resized between iterations of a running one.

use super::params;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Clone, Copy, Debug)]
pub(crate) struct TTEntry {
    pub score: i32,
    pub depth: i8,
    /// 0=EXACT, 1=LOWERBOUND (≥beta), 2=UPPERBOUND (≤alpha)
    pub flag: u8,
    pub generation: u8,
    pub best_move: u32,
    /// Position is in check (cached to avoid re-checking). Always false in
    /// Taikyoku (no check exists — SPEC §7.3); kept for chess-style engines.
    pub in_check: bool,
}

/// One table instance. Immutable once built, so an `Arc` swap under the write
/// lock is enough to "resize" while earlier readers finish their lookup.
struct TtStorage {
    slots: Box<[AtomicU64]>,
    /// Power-of-two bucket count; the hash is masked with `buckets - 1`.
    buckets: usize,
    mb: usize,
}

impl TtStorage {
    fn with_mb(mb: usize) -> Self {
        let mb = mb.clamp(params::TT_MIN_MB, params::TT_MAX_MB);
        let bytes = mb * 1024 * 1024;
        let want_slots =
            (bytes / std::mem::size_of::<AtomicU64>()).max(2 * params::TT_BUCKET_WIDTH);
        // Bucket count must be a power of two so the probe masks instead of
        // dividing. Round DOWN so we never allocate more than requested.
        let buckets = (want_slots / (params::TT_BUCKET_WIDTH * 2)).max(1);
        let buckets = 1usize << (usize::BITS - 1 - buckets.leading_zeros());
        // (1 << 62) buckets would overflow on the multiply below; the MiB
        // clamp already prevents that, but keep the guard explicit.
        let buckets = buckets.max(1);
        let slots: Box<[AtomicU64]> = (0..buckets * params::TT_BUCKET_WIDTH * 2)
            .map(|_| AtomicU64::new(0))
            .collect();
        TtStorage { slots, buckets, mb }
    }

    /// Actual size in MiB after the power-of-two rounding.
    fn actual_mb(&self) -> usize {
        (self.slots.len() * std::mem::size_of::<AtomicU64>()) / (1024 * 1024)
    }
}

static TT_STORAGE: OnceLock<RwLock<Arc<TtStorage>>> = OnceLock::new();

pub(crate) static TT_GEN: AtomicU64 = AtomicU64::new(1);

#[inline]
fn storage() -> &'static RwLock<Arc<TtStorage>> {
    TT_STORAGE.get_or_init(|| RwLock::new(Arc::new(TtStorage::with_mb(params::TT_DEFAULT_MB))))
}

/// Take a read guard, ignoring poisoning: a panicking search must not brick
/// the table for the rest of the process (the release profile is
/// `panic = "abort"`, but the test profile unwinds).
#[inline]
fn read_table() -> std::sync::RwLockReadGuard<'static, Arc<TtStorage>> {
    storage().read().unwrap_or_else(|e| e.into_inner())
}

#[inline]
fn bucket_index(hash: u64, buckets: usize) -> usize {
    ((hash as usize) & (buckets - 1)) * params::TT_BUCKET_WIDTH * 2
}

const TT_MOVE_MASK: u64 = (1 << 25) - 1;
const TT_CHECK_MASK: u64 = 1 << 25;

#[inline]
pub(crate) fn tt_pack(entry: &TTEntry, gen: u8) -> u64 {
    let sc = entry.score.clamp(-32000, 32000) as i16 as u16;
    let mv = entry.best_move & (TT_MOVE_MASK as u32);
    ((sc as u64) << 48)
        | ((entry.depth as u64 & 0x7F) << 41)
        | ((entry.flag as u64 & 0x03) << 39)
        | ((gen as u64) << 31)
        | if entry.in_check { TT_CHECK_MASK } else { 0 }
        | (mv as u64)
}

#[inline]
pub(crate) fn tt_unpack(packed: u64) -> TTEntry {
    TTEntry {
        score: ((packed >> 48) & 0xFFFF) as u16 as i16 as i32,
        depth: ((packed >> 41) & 0x7F) as i8,
        flag: ((packed >> 39) & 0x03) as u8,
        generation: ((packed >> 31) & 0xFF) as u8,
        best_move: (packed & TT_MOVE_MASK) as u32,
        in_check: (packed & TT_CHECK_MASK) != 0,
    }
}

/// Bump on every new `search()` call: entries from older generations lose
/// the replace race, achieving aging without clearing the table.
#[inline]
pub(crate) fn tt_gen() -> u8 {
    (TT_GEN.load(Ordering::Relaxed) & 0xFF) as u8
}

/// Advance the generation counter (called at the start of a new search).
pub(crate) fn tt_new_generation() {
    TT_GEN.fetch_add(1, Ordering::Relaxed);
}

/// Configure the table size in MiB. Returns the size actually allocated
/// (rounded down to a power-of-two bucket count). Safe to call at any time,
/// including while a search is running: readers hold an `Arc` to whichever
/// table was current when their lookup started.
pub(crate) fn resize_mb(mb: usize) -> usize {
    let table = Arc::new(TtStorage::with_mb(mb));
    let actual = table.actual_mb();
    {
        let mut guard = storage().write().unwrap_or_else(|e| e.into_inner());
        *guard = table;
    }
    // Fresh table: nothing is worth keeping, and an aged generation would
    // make the very first stores look like ancient history.
    tt_new_generation();
    actual
}

/// Current table size in MiB.
pub(crate) fn size_mb() -> usize {
    read_table().mb
}

/// Raw slot count (diagnostics / tests).
#[allow(dead_code)]
pub(crate) fn num_slots() -> usize {
    read_table().slots.len()
}

/// Clear every entry (new match / test isolation).
#[allow(dead_code)]
pub(crate) fn clear() {
    let table = read_table().clone();
    for slot in table.slots.iter() {
        slot.store(0, Ordering::Relaxed);
    }
    tt_new_generation();
}

pub(crate) fn tt_probe(hash: u64) -> Option<TTEntry> {
    let guard = read_table();
    let table: &TtStorage = guard.as_ref();
    let base = bucket_index(hash, table.buckets);
    let t: &[AtomicU64] = &table.slots;
    for i in 0..params::TT_BUCKET_WIDTH {
        let idx = base + i * 2;
        // ── Race-safe read ────────────────────────────────────────
        // Read hash with Acquire, snapshot the data, then RE-VERIFY the
        // hash: if it changed, the slot was overwritten mid-read and the
        // entry is treated as a miss.
        let stored = t[idx].load(Ordering::Acquire);
        if stored == hash {
            let entry = tt_unpack(t[idx + 1].load(Ordering::Acquire));
            if t[idx].load(Ordering::Acquire) != stored {
                continue; // slot overwritten mid-read — try next bucket slot
            }
            if entry.depth >= 0 { return Some(entry); }
        }
    }
    None
}

pub(crate) fn tt_store(hash: u64, entry: TTEntry) {
    // Pin BOTH the generation and the table for the whole write: a concurrent
    // `resize_mb` would otherwise install a fresh table whose generation
    // counter is unrelated to the entries we are comparing against.
    let gen = tt_gen();
    let guard = read_table();
    let table: &TtStorage = guard.as_ref();
    let base = bucket_index(hash, table.buckets);
    let t: &[AtomicU64] = &table.slots;
    let mut replace_idx = 0;
    let mut replace_score = i32::MAX;

    for i in 0..params::TT_BUCKET_WIDTH {
        let idx = base + i * 2;
        let old_hash = t[idx].load(Ordering::Relaxed);
        if old_hash == 0 {
            replace_idx = idx;
            break;
        }
        let old = tt_unpack(t[idx + 1].load(Ordering::Relaxed));
        let score = ((old.depth as i32) << 16) - (gen.wrapping_sub(old.generation) as i32);
        if score < replace_score {
            replace_score = score;
            replace_idx = idx;
        }
    }

    // ── Race-safe write ─────────────────────────────────────────
    // Publish data BEFORE the hash, both with Release: a reader that observes
    // the hash (Acquire) is guaranteed to see at least this data snapshot,
    // never a torn mix of old and new entries.
    t[replace_idx + 1].store(tt_pack(&entry, gen), Ordering::Release);
    t[replace_idx].store(hash, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tt_pack_unpack_roundtrip() {
        let entry = TTEntry {
            score: 1234,
            depth: 8,
            flag: 1,
            generation: 0, // ignored by tt_pack — the gen parameter wins
            best_move: 0x1234,
            in_check: true,
        };
        let out = tt_unpack(tt_pack(&entry, 7));
        assert_eq!(out.score, 1234);
        assert_eq!(out.depth, 8);
        assert_eq!(out.flag, 1);
        assert_eq!(out.generation, 7);
        assert_eq!(out.best_move, 0x1234);
        assert!(out.in_check);
        // Score clamping at the ±32000 boundary.
        let mut big = entry;
        big.score = 99_000;
        assert_eq!(tt_unpack(tt_pack(&big, 7)).score, 32_000);
        big.score = -99_000;
        assert_eq!(tt_unpack(tt_pack(&big, 7)).score, -32_000);
    }

    #[test]
    fn resize_rounds_down_to_a_power_of_two_and_keeps_working() {
        let before = size_mb();
        let actual = resize_mb(8);
        assert!(actual >= 4 && actual <= 8, "8 MiB request rounded to {} MiB", actual);
        assert_eq!(size_mb(), actual);
        // A store/probe round-trip must survive the resize.
        tt_store(0xDEAD_BEEF, TTEntry {
            score: 42, depth: 3, flag: 0, generation: 0, best_move: 7, in_check: false,
        });
        let hit = tt_probe(0xDEAD_BEEF).expect("entry must be found after resize");
        assert_eq!(hit.score, 42);
        assert_eq!(hit.best_move, 7);
        resize_mb(before);
        assert_eq!(size_mb(), before);
    }

    #[test]
    fn resize_clamps_absurd_requests() {
        let before = size_mb();
        resize_mb(0); // clamped up to the minimum
        assert!(size_mb() >= params::TT_MIN_MB);
        resize_mb(usize::MAX / 2); // clamped down to the maximum
        assert!(size_mb() <= params::TT_MAX_MB);
        resize_mb(before);
        assert_eq!(size_mb(), before);
    }
}
