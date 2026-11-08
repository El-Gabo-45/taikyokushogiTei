//! TKN1 v2 network file: parsing, validation and writing.
//!
//! Layout (little-endian, no padding, nothing after the last field):
//!
//! ```text
//! "TKN1" | u16 version=2 | [u8;32] arch_hash
//! | u32 kingbuckets=1 | u32 planes | u32 L1 | u32 out_buckets | u32 npiece
//! | u8  class_map[npiece] | u16 thresholds[out_buckets-1]
//! | i16 ft[dims*L1] | i32 ft_bias[L1] | i32 psqt[dims*out_buckets]
//! | i8 fc0[ob*L1*16] | i32 b0[ob*16] | i8 fc1[ob*16*32] | i32 b1[ob*32]
//! | i8 fc2[ob*32]    | i32 b2[ob]
//! ```
//!
//! Matrices are stored `[input][output]` (row-major), per output bucket.

use std::fmt;

use super::arch::{ArchSpec, NUM_THRESHOLDS};
use super::{Network, H1, H2, L1, MAX_BOARD_PIECES, NSQ, OUT_BUCKETS};

pub const MAGIC: &[u8; 4] = b"TKN1";
pub const VERSION: u16 = 2;

#[derive(Debug)]
pub enum NnueError {
    Io(std::io::Error),
    BadMagic,
    UnsupportedVersion(u16),
    /// The file was produced for a different architecture (class map,
    /// thresholds, dimensions or hash differ from this build).
    ArchMismatch(String),
    Truncated,
    TrailingBytes(usize),
    /// Weights could overflow the i32 accumulators.
    Overflow(String),
}

impl fmt::Display for NnueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NnueError::Io(e) => write!(f, "I/O error: {e}"),
            NnueError::BadMagic => write!(f, "not a TKN1 network file (bad magic)"),
            NnueError::UnsupportedVersion(v) => {
                write!(
                    f,
                    "unsupported network version {v} (only TKN1 v{VERSION} is accepted)"
                )
            }
            NnueError::ArchMismatch(m) => write!(f, "network does not match this build: {m}"),
            NnueError::Truncated => write!(f, "network file is truncated"),
            NnueError::TrailingBytes(n) => {
                write!(f, "network file has {n} unexpected trailing bytes")
            }
            NnueError::Overflow(m) => write!(f, "network weights can overflow: {m}"),
        }
    }
}

impl std::error::Error for NnueError {}

impl From<std::io::Error> for NnueError {
    fn from(e: std::io::Error) -> Self {
        NnueError::Io(e)
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], NnueError> {
        let end = self.pos.checked_add(n).ok_or(NnueError::Truncated)?;
        if end > self.buf.len() {
            return Err(NnueError::Truncated);
        }
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u16(&mut self) -> Result<u16, NnueError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, NnueError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i16s(&mut self, n: usize) -> Result<Box<[i16]>, NnueError> {
        Ok(self
            .take(n * 2)?
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect())
    }
    fn i32s(&mut self, n: usize) -> Result<Box<[i32]>, NnueError> {
        Ok(self
            .take(n * 4)?
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    }
    fn i8s(&mut self, n: usize) -> Result<Box<[i8]>, NnueError> {
        Ok(self.take(n)?.iter().map(|&b| b as i8).collect())
    }
}

fn mismatch<T>(what: impl Into<String>) -> Result<T, NnueError> {
    Err(NnueError::ArchMismatch(what.into()))
}

impl Network {
    /// Parse and validate a TKN1 v2 image against `arch`.
    pub fn from_bytes_with_arch(bytes: &[u8], arch: &ArchSpec) -> Result<Network, NnueError> {
        let mut r = Reader { buf: bytes, pos: 0 };
        if r.take(4)? != MAGIC {
            return Err(NnueError::BadMagic);
        }
        let ver = r.u16()?;
        if ver != VERSION {
            return Err(NnueError::UnsupportedVersion(ver));
        }
        let hash: [u8; 32] = r.take(32)?.try_into().unwrap();
        if hash != arch.arch_hash() {
            return mismatch(
                "architecture hash differs (class map, thresholds or layout changed; retrain and re-export)",
            );
        }
        let (kb, planes, l1, ob, npiece) = (
            r.u32()?,
            r.u32()? as usize,
            r.u32()? as usize,
            r.u32()? as usize,
            r.u32()? as usize,
        );
        if kb != 1 {
            return mismatch(format!("kingbuckets={kb}, expected 1"));
        }
        if planes != arch.planes() || l1 != L1 || ob != OUT_BUCKETS {
            return mismatch(format!("dimensions planes={planes} L1={l1} buckets={ob}"));
        }
        if npiece != arch.class_map.len() {
            return mismatch(format!(
                "class map has {npiece} entries, expected {}",
                arch.class_map.len()
            ));
        }
        let class_map = r.take(npiece)?.to_vec();
        if class_map != arch.class_map {
            return mismatch("class map differs from rules/nnue_arch.json");
        }
        let mut thresholds = [0u16; NUM_THRESHOLDS];
        for t in thresholds.iter_mut() {
            *t = r.u16()?;
        }
        if thresholds.as_slice() != arch.thresholds_pieces.as_slice() {
            return mismatch("output-bucket thresholds differ from rules/nnue_arch.json");
        }

        let dims = planes * NSQ;
        let ft = r.i16s(dims * L1)?;
        let ft_bias = r.i32s(L1)?;
        let psqt = r.i32s(dims * OUT_BUCKETS)?;
        let fc0_w = r.i8s(OUT_BUCKETS * L1 * H1)?;
        let fc0_b = r.i32s(OUT_BUCKETS * H1)?;
        let fc1_w = r.i8s(OUT_BUCKETS * H1 * H2)?;
        let fc1_b = r.i32s(OUT_BUCKETS * H2)?;
        let fc2_w = r.i8s(OUT_BUCKETS * H2)?;
        let fc2_b = r.i32s(OUT_BUCKETS)?;
        if r.pos != bytes.len() {
            return Err(NnueError::TrailingBytes(bytes.len() - r.pos));
        }

        // The accumulators are i32: prove they cannot overflow for any board.
        let max_ft = ft.iter().map(|&w| (w as i64).abs()).max().unwrap_or(0);
        let max_fb = ft_bias.iter().map(|&w| (w as i64).abs()).max().unwrap_or(0);
        if MAX_BOARD_PIECES as i64 * max_ft + max_fb >= 1 << 31 {
            return Err(NnueError::Overflow(
                "feature-transformer accumulator".into(),
            ));
        }
        // PSQT: at most MAX_BOARD_PIECES features are active, so the worst case
        // of one bucket is the sum of its largest MAX_BOARD_PIECES magnitudes.
        // It must stay below 2^30 so that the difference of the two
        // perspectives also fits in an i32.
        for b in 0..OUT_BUCKETS {
            let mut col: Vec<i64> = psqt
                .iter()
                .skip(b)
                .step_by(OUT_BUCKETS)
                .map(|&w| (w as i64).abs())
                .collect();
            let k = MAX_BOARD_PIECES.min(col.len());
            col.select_nth_unstable_by(k - 1, |a, b| b.cmp(a));
            if col[..k].iter().sum::<i64>() >= 1 << 30 {
                return Err(NnueError::Overflow(format!(
                    "PSQT accumulator of bucket {b}"
                )));
            }
        }

        Ok(Network {
            planes,
            class_map,
            thresholds,
            arch_hash: hash,
            eval_limit: arch.eval_limit,
            tt_salt: super::next_tt_salt(),
            ft,
            ft_bias,
            psqt,
            fc0_w,
            fc0_b,
            fc1_w,
            fc1_b,
            fc2_w,
            fc2_b,
        })
    }

    /// Parse a TKN1 v2 image against the architecture compiled into this build.
    pub fn from_bytes(bytes: &[u8]) -> Result<Network, NnueError> {
        Network::from_bytes_with_arch(bytes, ArchSpec::embedded())
    }

    /// Load a network file. Never falls back to random weights: any problem
    /// is reported as an error.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Network, NnueError> {
        Network::from_bytes(&std::fs::read(path)?)
    }

    /// Serialise to the TKN1 v2 layout.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(self.ft.len() * 2 + self.psqt.len() * 4 + 4096);
        o.extend_from_slice(MAGIC);
        o.extend_from_slice(&VERSION.to_le_bytes());
        o.extend_from_slice(&self.arch_hash);
        for v in [
            1u32,
            self.planes as u32,
            L1 as u32,
            OUT_BUCKETS as u32,
            self.class_map.len() as u32,
        ] {
            o.extend_from_slice(&v.to_le_bytes());
        }
        o.extend_from_slice(&self.class_map);
        for t in &self.thresholds {
            o.extend_from_slice(&t.to_le_bytes());
        }
        for w in self.ft.iter() {
            o.extend_from_slice(&w.to_le_bytes());
        }
        for w in self.ft_bias.iter().chain(self.psqt.iter()) {
            o.extend_from_slice(&w.to_le_bytes());
        }
        o.extend(self.fc0_w.iter().map(|&w| w as u8));
        for w in self.fc0_b.iter() {
            o.extend_from_slice(&w.to_le_bytes());
        }
        o.extend(self.fc1_w.iter().map(|&w| w as u8));
        for w in self.fc1_b.iter() {
            o.extend_from_slice(&w.to_le_bytes());
        }
        o.extend(self.fc2_w.iter().map(|&w| w as u8));
        for w in self.fc2_b.iter() {
            o.extend_from_slice(&w.to_le_bytes());
        }
        o
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), NnueError> {
        std::fs::write(path, self.to_bytes())?;
        Ok(())
    }
}
