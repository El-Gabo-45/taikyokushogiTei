use super::*;
use crate::eval::nnue::format::{MAGIC, VERSION, ZSTD_MAGIC};
use crate::Board as PubBoard;

fn net() -> Arc<Network> {
    Arc::new(Network::random(42))
}

#[test]
fn embedded_arch_is_valid() {
    let a = ArchSpec::embedded();
    assert_eq!(a.planes(), 2 * a.num_classes);
    assert_eq!(a.thresholds_pieces.len(), OUT_BUCKETS - 1);
    // A 32-byte hash that is stable for the same input.
    assert_eq!(a.arch_hash(), a.arch_hash());
}

#[test]
fn class_map_matches_frozen_arch() {
    // If this fails, pieces.rs changed: run `cargo run --bin nnue -- gen-classes`,
    // then retrain (the class map is part of the architecture hash).
    let generated = classes::generate_class_map();
    assert_eq!(generated, ArchSpec::embedded().class_map);
}

#[test]
fn class_values_match_frozen_arch() {
    let a = ArchSpec::embedded();
    assert_eq!(classes::class_values(&a.class_map), a.class_values);
}

#[test]
fn class_count_is_in_range_and_every_piece_has_a_class() {
    let map = classes::generate_class_map();
    let c = classes::num_classes(&map);
    assert!((24..=32).contains(&c), "{c} classes");
    // every class has at least one member
    for cls in 0..c {
        assert!(
            map.iter().skip(1).any(|&x| x as usize == cls),
            "empty class {cls}"
        );
    }
    // royals have a class of their own
    let k = crate::pieces::find_by_abbrev("K").unwrap();
    let cp = crate::pieces::find_by_abbrev("CP").unwrap();
    assert_eq!(map[k as usize], map[cp as usize]);
    let royal_members = (1..map.len())
        .filter(|&p| map[p] == map[k as usize])
        .count();
    assert_eq!(royal_members, 2);
}

#[test]
fn white_perspective_is_a_180_degree_rotation() {
    let n = Network::random(1);
    for &(r, c) in &[(0usize, 0usize), (3, 17), (35, 35), (12, 30), (20, 5)] {
        let sq = r * 36 + c;
        let rot = (35 - r) * 36 + (35 - c);
        for pt in [1u16, 20, 301] {
            // White's view of a square equals Black's view of the rotated square.
            assert_eq!(
                n.feature_index(WHITE, pt, WHITE, sq),
                n.feature_index(BLACK, pt, BLACK, rot)
            );
            assert_eq!(
                n.feature_index(WHITE, pt, BLACK, sq),
                n.feature_index(BLACK, pt, WHITE, rot)
            );
        }
    }
}

#[test]
fn feature_indices_are_in_range_and_unique_per_position() {
    let n = Network::random(1);
    let mut b = PubBoard::initial();
    b.set_network(Some(Arc::new(n)));
    let net = b.network().unwrap().clone();
    let d = dump::dump_position(&b, &net);
    for p in 0..2 {
        assert_eq!(d.features[p].len(), d.pieces.len());
        assert!(d.features[p].iter().all(|&f| (f as usize) < net.dims()));
        assert!(
            d.features[p].windows(2).all(|w| w[0] < w[1]),
            "duplicate feature"
        );
    }
}

#[test]
fn bucket_boundaries() {
    let n = Network::random(1);
    let t = n.thresholds().to_vec();
    assert_eq!(n.bucket(0), 0);
    assert_eq!(n.bucket(t[0] as usize - 1), 0);
    assert_eq!(n.bucket(t[0] as usize), 1);
    assert_eq!(n.bucket(t[6] as usize - 1), 6);
    assert_eq!(n.bucket(t[6] as usize), 7);
    assert_eq!(n.bucket(804), 7);
}

#[test]
fn file_roundtrip_is_exact() {
    let n = Network::random(5);
    let bytes = n.to_bytes();
    let back = Network::from_bytes(&bytes).expect("roundtrip");
    assert_eq!(back.to_bytes(), bytes);
}

#[test]
fn zstd_compressed_network_loads_transparently() {
    let n = Network::random(5);
    let raw = n.to_bytes();
    let compressed = zstd::stream::encode_all(&raw[..], 3).expect("zstd encode");
    assert!(compressed.starts_with(&ZSTD_MAGIC));

    let back = Network::from_bytes(&compressed).expect("load zstd-compressed network");
    assert_eq!(back.to_bytes(), raw);
}

#[test]
fn zstd_compressed_network_file_loads_through_load() {
    let n = Network::random(6);
    let raw = n.to_bytes();
    let compressed = zstd::stream::encode_all(&raw[..], 3).expect("zstd encode");
    let path = std::env::temp_dir().join(format!(
        "taikyokushogi-net-{}-{}.tkn.zst",
        std::process::id(),
        line!()
    ));
    std::fs::write(&path, compressed).expect("write temp network");

    let loaded = Network::load(&path);
    let _ = std::fs::remove_file(&path);
    let back = loaded.expect("load compressed file");
    assert_eq!(back.to_bytes(), raw);
}

#[test]
fn zstd_loader_rejects_corrupt_frames() {
    let mut bad = ZSTD_MAGIC.to_vec();
    bad.extend_from_slice(&[0u8; 16]);
    assert!(matches!(
        Network::from_bytes(&bad),
        Err(NnueError::Decompression(_))
    ));
}

#[test]
fn zstd_loader_still_validates_tkn1() {
    let not_a_network =
        zstd::stream::encode_all(&b"not a TKN1 network"[..], 3).expect("zstd encode");
    assert!(matches!(
        Network::from_bytes(&not_a_network),
        Err(NnueError::BadMagic)
    ));
}

#[test]
fn loader_rejects_bad_files() {
    let good = Network::random(5).to_bytes();

    let mut bad = good.clone();
    bad[0] = b'X';
    assert!(matches!(
        Network::from_bytes(&bad),
        Err(NnueError::BadMagic)
    ));

    let mut old = good.clone();
    old[..4].copy_from_slice(b"NNU1");
    assert!(matches!(
        Network::from_bytes(&old),
        Err(NnueError::BadMagic)
    ));

    let mut v1 = good.clone();
    v1[4..6].copy_from_slice(&(VERSION - 1).to_le_bytes());
    assert!(matches!(
        Network::from_bytes(&v1),
        Err(NnueError::UnsupportedVersion(_))
    ));

    assert!(matches!(
        Network::from_bytes(&good[..good.len() - 1]),
        Err(NnueError::Truncated)
    ));
    assert!(matches!(
        Network::from_bytes(&good[..10]),
        Err(NnueError::Truncated)
    ));

    let mut extra = good.clone();
    extra.push(0);
    assert!(matches!(
        Network::from_bytes(&extra),
        Err(NnueError::TrailingBytes(1))
    ));

    // flipped hash byte
    let mut h = good.clone();
    h[6] ^= 0xFF;
    assert!(matches!(
        Network::from_bytes(&h),
        Err(NnueError::ArchMismatch(_))
    ));

    // header layout: magic(4) ver(2) hash(32) 5*u32(20) -> class_map starts at 58
    let mut cm = good.clone();
    cm[58 + 1] ^= 1;
    assert!(matches!(
        Network::from_bytes(&cm),
        Err(NnueError::ArchMismatch(_))
    ));

    // thresholds start after the class map
    let tpos = 58 + ArchSpec::embedded().class_map.len();
    let mut th = good.clone();
    th[tpos] ^= 1;
    assert!(matches!(
        Network::from_bytes(&th),
        Err(NnueError::ArchMismatch(_))
    ));

    assert_eq!(&good[..4], MAGIC);
}

#[test]
fn loader_rejects_possible_overflow() {
    let mut n = Network::random(5);
    n.ft[0] = i16::MAX;
    for w in n.ft.iter_mut().take(L1 * 400) {
        *w = i16::MAX;
    }
    // 820 * 32767 = 26.8M, far below 2^31: still fine
    assert!(Network::from_bytes(&n.to_bytes()).is_ok());
    // A single huge PSQT weight is fine (few pieces can be on one feature)...
    n.psqt[0] = 1 << 29;
    assert!(Network::from_bytes(&n.to_bytes()).is_ok());
    // ...but hundreds of them could overflow the i32 accumulator.
    for w in n.psqt.iter_mut().step_by(OUT_BUCKETS).take(900) {
        *w = 1 << 29;
    }
    assert!(matches!(
        Network::from_bytes(&n.to_bytes()),
        Err(NnueError::Overflow(_))
    ));
}

fn assert_acc_matches_refresh(b: &PubBoard, what: &str) {
    let st = b.inner.nnue.as_deref().expect("network attached");
    let mut fresh = Accumulator::zero();
    fresh.refresh(&st.net, &b.inner);
    assert!(st.acc == fresh, "accumulator diverged: {what}");
}

/// Deterministic walk: tactical moves preferred, regular undos, so captures,
/// promotions, igui, lion jumps and range captures all hit the delta paths.
#[test]
fn incremental_accumulator_matches_refresh_over_a_long_walk() {
    let mut b = PubBoard::initial();
    b.set_network(Some(net()));
    let mut state = 12345u64;
    let mut next = |n: usize| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 33) as usize) % n
    };
    let mut seen_tactical = 0usize;
    // [capture, lion mid-capture, range capture, promotion, igui]
    let mut kinds = [0usize; 5];
    for ply in 0..260 {
        assert_acc_matches_refresh(&b, &format!("ply {ply}"));
        let moves = b.legal_moves();
        if moves.is_empty() || b.game_result().is_some() {
            break;
        }
        let tactical: Vec<usize> = (0..moves.len())
            .filter(|&i| {
                let r = moves[i].raw();
                r.captured_piece != 0 || r.mid_piece != 0 || r.range_cap || r.promotion || r.is_igui
            })
            .collect();
        let i = if !tactical.is_empty() && next(3) != 0 {
            seen_tactical += 1;
            tactical[next(tactical.len())]
        } else {
            next(moves.len())
        };
        {
            let r = moves[i].raw();
            kinds[0] += (r.captured_piece != 0) as usize;
            kinds[1] += (r.mid_piece != 0) as usize;
            kinds[2] += r.range_cap as usize;
            kinds[3] += r.promotion as usize;
            kinds[4] += r.is_igui as usize;
        }
        b.apply(&moves[i]);
        if ply % 9 == 4 {
            assert!(b.undo());
            assert_acc_matches_refresh(&b, &format!("after undo at ply {ply}"));
            b.apply(&moves[i]);
        }
    }
    eprintln!("walk move kinds [capture, mid, range, promo, igui] = {kinds:?}");
    assert!(seen_tactical > 5, "walk must exercise tactical moves");
    assert!(kinds[0] > 0, "walk must include plain captures");
}

/// Undoing further back than the snapshot ring must refresh from the board
/// and still be exact; memory must not depend on the game length.
#[test]
fn deep_undo_beyond_the_ring_is_exact() {
    let mut b = PubBoard::initial();
    b.set_network(Some(net()));
    let mut state = 99u64;
    let mut played = 0usize;
    for _ in 0..(SNAPSHOT_RING + 40) {
        let moves = b.legal_moves();
        if moves.is_empty() || b.game_result().is_some() {
            break;
        }
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        b.apply(&moves[((state >> 33) as usize) % moves.len()]);
        played += 1;
    }
    assert!(
        played > SNAPSHOT_RING,
        "need more plies than the ring ({played})"
    );
    for k in 0..played {
        assert!(b.undo());
        if k % 37 == 0 || k + 1 == played || k == SNAPSHOT_RING - 1 || k == SNAPSHOT_RING {
            assert_acc_matches_refresh(&b, &format!("undo #{k}"));
        }
    }
    assert_acc_matches_refresh(&b, "back at the start");
}

#[test]
fn null_move_keeps_the_accumulator() {
    let mut b = PubBoard::initial();
    b.set_network(Some(net()));
    let before = b.inner.nnue.as_deref().unwrap().acc;
    let e_black = b.evaluate();
    b.inner.null_move();
    assert!(b.inner.nnue.as_deref().unwrap().acc == before);
    b.inner.undo_null_move();
    assert!(b.inner.nnue.as_deref().unwrap().acc == before);
    assert_eq!(b.evaluate(), e_black);
}

#[test]
fn boards_with_different_networks_do_not_interfere() {
    let (n1, n2) = (Arc::new(Network::random(1)), Arc::new(Network::random(2)));
    let mut a = PubBoard::initial();
    let mut b = PubBoard::initial();
    a.set_network(Some(n1.clone()));
    b.set_network(Some(n2.clone()));
    let (ea, eb) = (a.evaluate(), b.evaluate());
    assert_ne!(ea, eb, "different random networks should disagree");
    // same results regardless of evaluation order / interleaved moves
    let m = a.legal_moves()[0];
    a.apply(&m);
    assert_eq!(b.evaluate(), eb);
    a.undo();
    assert_eq!(a.evaluate(), ea);
    // detaching returns to the hand-crafted evaluator
    b.set_network(None);
    assert!(b.network().is_none());
    assert_eq!(b.evaluate(), PubBoard::initial().evaluate());
}

#[test]
fn tt_key_separates_evaluators() {
    let (n1, n2) = (Arc::new(Network::random(1)), Arc::new(Network::random(1)));
    let classic = PubBoard::initial();
    let mut a = PubBoard::initial();
    let mut a2 = PubBoard::initial();
    let mut b = PubBoard::initial();
    a.set_network(Some(n1.clone()));
    a2.set_network(Some(n1.clone()));
    b.set_network(Some(n2)); // same weights, different instance
    let (kc, ka, ka2, kb) = (
        classic.inner.tt_key(),
        a.inner.tt_key(),
        a2.inner.tt_key(),
        b.inner.tt_key(),
    );
    assert_eq!(
        a.inner.hash, classic.inner.hash,
        "hash itself must not change"
    );
    assert_eq!(ka, ka2, "boards sharing a network share TT entries");
    assert_ne!(ka, kc);
    assert_ne!(ka, kb);
    assert_ne!(kb, kc);
    // keys stay consistent through apply/undo
    let m = a.legal_moves()[0];
    a.apply(&m);
    a.undo();
    assert_eq!(a.inner.tt_key(), ka);
}

#[test]
fn set_network_with_same_arc_keeps_state() {
    let n = net();
    let mut b = PubBoard::initial();
    b.set_network(Some(n.clone()));
    let m = b.legal_moves()[0];
    b.apply(&m);
    let acc = b.inner.nnue.as_deref().unwrap().acc;
    b.set_network(Some(n.clone())); // same Arc: no refresh needed, state valid
    assert!(b.inner.nnue.as_deref().unwrap().acc == acc);
    assert_acc_matches_refresh(&b, "after re-attaching the same network");
}

#[test]
fn evaluate_is_clamped() {
    let mut n = Network::random(3);
    n.eval_limit = 5;
    let mut b = PubBoard::initial();
    b.set_network(Some(Arc::new(n)));
    assert!(b.evaluate().abs() <= 5);
}

#[test]
fn state_from_tsfen_matches_played_position() {
    let n = net();
    let mut b = PubBoard::initial();
    b.set_network(Some(n.clone()));
    for _ in 0..6 {
        let m = b.legal_moves()[3];
        b.apply(&m);
    }
    let mut c = PubBoard::from_tsfen(&b.to_tsfen()).unwrap();
    c.set_network(Some(n));
    assert_eq!(b.evaluate(), c.evaluate());
}

/// Exhaustive check of the rare move kinds: for every piece type with area
/// movement (lion double steps) or igui, build a position with the piece
/// surrounded by enemy and friendly pieces, then apply and undo EVERY legal
/// move, comparing the incremental accumulator with a refresh each time.
#[test]
fn lion_and_igui_moves_keep_the_accumulator_exact() {
    use crate::board::Board as Inner;
    let abbr = |a: &str| crate::pieces::find_by_abbrev(a).unwrap_or_else(|| panic!("no piece {a}"));
    let (pawn, king, gold, rook) = (abbr("P"), abbr("K"), abbr("G"), abbr("R"));
    let n = net();
    let mut kinds = [0usize; 5]; // capture, mid, range, promotion, igui
    let mut tested_pieces = 0usize;
    let mut total_moves = 0usize;

    for pt in 1..=crate::pieces::num_piece_types() as u16 {
        let m = crate::pieces::movement(pt);
        if !(m.area > 0 || m.igui) {
            continue;
        }
        tested_pieces += 1;
        let mut b = Inner::new();
        let put = |b: &mut Inner, r: usize, c: usize, p: u16, col: u8| {
            b.cells[r * 36 + c] = make_cell(p, col);
        };
        put(&mut b, 35, 0, king, BLACK);
        put(&mut b, 0, 35, king, WHITE);
        put(&mut b, 18, 18, pt, BLACK);
        // enemy ring at distance 1 and 2 (some squares left empty), friends around
        for (dr, dc, p, col) in [
            (-1i32, 0i32, pawn, WHITE),
            (0, 1, gold, WHITE),
            (1, 1, pawn, WHITE),
            (-2, 0, pawn, WHITE),
            (0, -2, rook, WHITE),
            (2, 2, gold, WHITE),
            (-1, -1, pawn, BLACK),
            (1, 0, gold, BLACK),
            (0, -1, pawn, WHITE),
            (-2, 2, pawn, WHITE),
            (2, -1, pawn, WHITE),
            (1, -2, pawn, BLACK),
        ] {
            put(&mut b, (18 + dr) as usize, (18 + dc) as usize, p, col);
        }
        // a few pieces on far rows so promotions are reachable
        put(&mut b, 3, 18, pt, BLACK);
        put(&mut b, 2, 17, pawn, WHITE);
        b.rebuild_lists_pub();
        b.set_network(Some(n.clone()));

        let moves = crate::movegen::generate_legal_moves(&b);
        for mv in &moves {
            kinds[0] += (mv.captured_piece != 0) as usize;
            kinds[1] += (mv.mid_piece != 0) as usize;
            kinds[2] += mv.range_cap as usize;
            kinds[3] += mv.promotion as usize;
            kinds[4] += mv.is_igui as usize;
            total_moves += 1;
            b.apply_move(mv);
            let st = b.nnue.as_deref().unwrap();
            let mut fresh = Accumulator::zero();
            fresh.refresh(&st.net, &b);
            assert!(st.acc == fresh, "diverged after piece {pt} move {mv:?}");
            b.undo_move();
            let st = b.nnue.as_deref().unwrap();
            let mut fresh = Accumulator::zero();
            fresh.refresh(&st.net, &b);
            assert!(
                st.acc == fresh,
                "diverged after undo of piece {pt} move {mv:?}"
            );
        }
    }
    eprintln!("lion/igui exhaustive: {tested_pieces} piece types, {total_moves} moves, kinds [capture, mid, range, promo, igui] = {kinds:?}");
    assert!(tested_pieces >= 4);
    assert!(kinds[1] > 0, "no lion mid-capture was exercised");
    assert!(kinds[4] > 0, "no igui was exercised");
}
