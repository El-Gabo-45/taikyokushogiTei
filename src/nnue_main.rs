//! `nnue` — maintenance tool for the NNUE architecture and network files.
//!
//! ```text
//! nnue classes                              print the piece-class table
//! nnue gen-classes [--arch F] [--thresholds a,b,c,d,e,f,g]
//!                                           regenerate rules/nnue_arch.json
//! nnue info <net>                           header, dimensions, hash
//! nnue verify <net>                         load + incremental==refresh check
//! nnue random-net <out> [--seed N]          deterministic random network (tests)
//! nnue dump-positions --net F [--n N] [--seed S] [--thin] [--out F]
//!                                           JSONL for training/parity_gate.py
//!                                           (--thin removes random pieces so every
//!                                           output bucket is exercised)
//! nnue bench [<net>]                        evals/s, apply+undo overhead
//! ```
//!
//! A `<net>` argument can be a raw `.tkn` or a zstd-compressed `.tkn.zst`;
//! `Network::load` detects the format from the file contents.

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Instant;

use taikyokushogi::nnue::{
    arch::ArchSpec,
    classes,
    dump::{dump_position, thinned},
    format::ZSTD_MAGIC,
};
use taikyokushogi::{Board, Network};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn die(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(2);
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

fn load(path: &str) -> Network {
    Network::load(path).unwrap_or_else(|e| die(format!("cannot load `{path}`: {e}")))
}

/// One step of a seeded random walk (captures preferred half of the time,
/// occasional undo). Returns false when the game is over.
fn step(board: &mut Board, rng: &mut Rng) -> bool {
    if board.game_result().is_some() {
        return false;
    }
    if rng.below(100) < 12 && board.undo() {
        return true;
    }
    let moves = board.legal_moves();
    if moves.is_empty() {
        return false;
    }
    let tactical: Vec<usize> = (0..moves.len())
        .filter(|&i| {
            let r = moves[i].raw();
            r.captured_piece != 0 || r.mid_piece != 0 || r.range_cap || r.promotion || r.is_igui
        })
        .collect();
    let i = if !tactical.is_empty() && rng.below(2) == 0 {
        tactical[rng.below(tactical.len())]
    } else {
        rng.below(moves.len())
    };
    board.apply(&moves[i]);
    true
}

fn cmd_classes() {
    let map = classes::generate_class_map();
    print!("{}", classes::describe(&map));
    println!("{} classes", classes::num_classes(&map));
}

fn cmd_gen_classes(args: &[String]) {
    let path = flag(args, "--arch").unwrap_or("rules/nnue_arch.json");
    let old: serde_json::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::Value::Null);
    let thresholds: Vec<u64> = match flag(args, "--thresholds") {
        Some(t) => t
            .split(',')
            .map(|x| x.trim().parse().unwrap_or_else(|_| die("bad --thresholds")))
            .collect(),
        None => old["thresholds_pieces"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
            .filter(|v: &Vec<u64>| v.len() == 7)
            .unwrap_or_else(|| vec![102, 203, 304, 405, 506, 607, 708]),
    };
    let eval_limit = old["eval_limit"].as_i64().unwrap_or(250_000);
    let map = classes::generate_class_map();
    let cm: Vec<String> = map.iter().map(|c| c.to_string()).collect();
    let th: Vec<String> = thresholds.iter().map(|t| t.to_string()).collect();
    let json = format!(
        "{{\n  \"format\": \"TKN1\",\n  \"version\": 2,\n  \"num_squares\": 1296,\n  \"l1\": 256,\n  \"out_buckets\": 8,\n  \"ft_shift\": 6,\n  \"fv_scale\": 16,\n  \"eval_limit\": {},\n  \"num_classes\": {},\n  \"class_map\": [{}],\n  \"class_values\": [{}],\n  \"thresholds_pieces\": [{}]\n}}\n",
        eval_limit,
        classes::num_classes(&map),
        cm.join(", "),
        classes::class_values(&map).iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "),
        th.join(", ")
    );
    ArchSpec::from_json(&json).unwrap_or_else(|e| die(format!("generated arch is invalid: {e}")));
    std::fs::write(path, json).unwrap_or_else(|e| die(format!("cannot write {path}: {e}")));
    println!(
        "wrote {path} ({} classes, thresholds {:?})",
        classes::num_classes(&map),
        thresholds
    );
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn cmd_info(path: &str) {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let compressed = std::fs::File::open(path)
        .and_then(|mut f| {
            let mut magic = [0u8; 4];
            f.read_exact(&mut magic)?;
            Ok(magic == ZSTD_MAGIC)
        })
        .unwrap_or(false);
    let net = load(path);
    println!("file        {path} ({:.1} MB)", size as f64 / 1e6);
    println!(
        "format      TKN1 v2{}",
        if compressed { " (zstd-compressed)" } else { "" }
    );
    println!("arch hash   {}", hex(net.arch_hash()));
    println!(
        "classes     {} (planes {}, features {})",
        net.num_classes(),
        net.planes(),
        net.dims()
    );
    println!(
        "L1          {}  buckets {}  thresholds {:?}",
        taikyokushogi::nnue::L1,
        taikyokushogi::nnue::OUT_BUCKETS,
        net.thresholds()
    );
    println!("descriptor  {}", ArchSpec::embedded().descriptor());
}

fn cmd_verify(path: &str) {
    let net = Arc::new(load(path));
    let mut board = Board::initial();
    board.set_network(Some(net.clone()));
    let mut rng = Rng(7);
    let mut checked = 0;
    for _ in 0..60 {
        let inc = board.evaluate();
        let mut fresh = Board::from_tsfen(&board.to_tsfen()).unwrap_or_else(|e| die(e));
        fresh.set_network(Some(net.clone()));
        let full = fresh.evaluate();
        if inc != full {
            die(format!(
                "incremental {inc} != refresh {full} after {checked} positions"
            ));
        }
        checked += 1;
        if !step(&mut board, &mut rng) {
            break;
        }
    }
    println!("OK: `{path}` loads; incremental == refresh on {checked} positions");
}

fn cmd_dump(args: &[String]) {
    let net_path = flag(args, "--net").unwrap_or_else(|| die("--net is required"));
    let n: usize = flag(args, "--n").map_or(500, |v| v.parse().unwrap_or_else(|_| die("bad --n")));
    let seed: u64 =
        flag(args, "--seed").map_or(1, |v| v.parse().unwrap_or_else(|_| die("bad --seed")));
    let net = Arc::new(load(net_path));
    let mut out: Box<dyn Write> = match flag(args, "--out") {
        Some(p) => Box::new(std::io::BufWriter::new(
            std::fs::File::create(p).unwrap_or_else(|e| die(format!("cannot create {p}: {e}"))),
        )),
        None => Box::new(std::io::stdout()),
    };
    let mut rng = Rng(seed);
    let mut board = Board::initial();
    board.set_network(Some(net.clone()));
    let mut written = 0usize;
    let mut guard = 0usize;
    while written < n {
        guard += 1;
        if guard > n * 50 {
            die("random walk keeps ending; use a different --seed");
        }
        if !step(&mut board, &mut rng) {
            board = Board::initial();
            board.set_network(Some(net.clone()));
            continue;
        }
        if board.game_result().is_some() {
            continue;
        }
        let thin = args.iter().any(|a| a == "--thin");
        let d = if thin {
            // Keep between 2% and 100% of the non-royal pieces.
            let mut t = thinned(&board, 20 + rng.below(981) as u32, rng.next());
            t.set_network(Some(net.clone()));
            if t.game_result().is_some() {
                continue;
            }
            dump_position(&t, &net)
        } else {
            dump_position(&board, &net)
        };
        let pieces: Vec<serde_json::Value> = d
            .pieces
            .iter()
            .map(|&(s, p, c)| serde_json::json!([s, p, c]))
            .collect();
        let line = serde_json::json!({
            "stm": d.stm, "bucket": d.bucket, "eval": d.eval,
            "pieces": pieces, "features": [d.features[0], d.features[1]],
        });
        writeln!(out, "{line}").unwrap_or_else(|e| die(e));
        written += 1;
    }
    out.flush().ok();
    eprintln!("wrote {written} positions");
}

fn cmd_bench(path: Option<&str>) {
    let net = Arc::new(path.map_or_else(|| Network::random(1), load));
    let mut board = Board::initial();
    let mut rng = Rng(3);
    for _ in 0..40 {
        if !step(&mut board, &mut rng) {
            break;
        }
    }
    board.set_network(Some(net.clone()));
    let iters = 20_000u32;
    let t = Instant::now();
    let mut acc = 0i64;
    for _ in 0..iters {
        acc += board.evaluate() as i64;
    }
    let ev = t.elapsed().as_secs_f64() / iters as f64;

    let t = Instant::now();
    let refreshes = 200u32;
    for _ in 0..refreshes {
        board.set_network(None);
        board.set_network(Some(net.clone()));
    }
    let rf = t.elapsed().as_secs_f64() / refreshes as f64;

    let moves = board.legal_moves();
    let t = Instant::now();
    let au_iters = 2_000u32;
    for i in 0..au_iters {
        board.apply(&moves[i as usize % moves.len()]);
        board.undo();
    }
    let au_nnue = t.elapsed().as_secs_f64() / au_iters as f64;
    board.set_network(None);
    let t = Instant::now();
    for i in 0..au_iters {
        board.apply(&moves[i as usize % moves.len()]);
        board.undo();
    }
    let au_classic = t.elapsed().as_secs_f64() / au_iters as f64;

    println!(
        "evaluate()  incremental {:8.2} us  ({:.0} evals/s)",
        ev * 1e6,
        1.0 / ev
    );
    println!("full refresh            {:8.2} us", rf * 1e6);
    println!("apply+undo  with NNUE   {:8.2} us", au_nnue * 1e6);
    println!("apply+undo  classic     {:8.2} us", au_classic * 1e6);
    if acc == i64::MIN {
        println!();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = if args.is_empty() { &[][..] } else { &args[1..] };
    match args.first().map(|s| s.as_str()) {
        Some("classes") => cmd_classes(),
        Some("gen-classes") => cmd_gen_classes(rest),
        Some("info") => cmd_info(
            rest.first()
                .unwrap_or_else(|| die("usage: nnue info <net>")),
        ),
        Some("verify") => cmd_verify(
            rest.first()
                .unwrap_or_else(|| die("usage: nnue verify <net>")),
        ),
        Some("random-net") => {
            let out = rest
                .first()
                .unwrap_or_else(|| die("usage: nnue random-net <out> [--seed N]"));
            let seed: u64 =
                flag(rest, "--seed").map_or(1, |v| v.parse().unwrap_or_else(|_| die("bad --seed")));
            Network::random(seed).save(out).unwrap_or_else(|e| die(e));
            println!("wrote {out} (random weights, seed {seed}; NOT a playing network)");
        }
        Some("dump-positions") => cmd_dump(rest),
        Some("bench") => cmd_bench(rest.first().map(|s| s.as_str())),
        _ => {
            eprintln!(
                "usage: nnue <classes|gen-classes|info|verify|random-net|dump-positions|bench> ..."
            );
            std::process::exit(if args.is_empty() { 0 } else { 2 });
        }
    }
}
