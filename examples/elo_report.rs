//! Measure and display the engine's **current playing strength** as a
//! Stockfish-style self-referential Elo ladder.
//!
//! The engine has no external rating pool to anchor to, so "current Elo" is
//! measured on a **depth ladder**: depth `d` plays depth `d-1` (seeded random
//! openings, alternating colors) and the pairwise Elo gains accumulate into
//! one rating scale anchored at depth 1 = 0. The ladder is reproducible:
//! same invocation → same games.
//!
//! Usage:
//!
//! ```text
//! cargo run --release --example elo_report -- [flags]
//!   --games N       games per ladder rung (default 40)
//!   --max-depth D   top of the ladder (default 3; depth 4+ gets slow)
//!   --movetime MS   per-move time cap for both sides (default 300; must be
//!                   above what the top depth needs, else that rung is
//!                   truncated by the clock instead of the depth)
//!   --maxplies P    ply cap per game, longer games draw (default 250)
//!   --open K        random opening plies per game (default 8)
//!   --threads N     Lazy SMP workers per side (default 1; 1 is reproducible)
//!   --net PATH      run the whole ladder with this NNUE network attached
//!   --json PATH     write a machine-readable report (for scripts/CI)
//!   --md PATH       update the <!-- elo-report:begin/end --> block in a
//!                   markdown file (e.g. docs/STRENGTH.md)
//! ```
//!
//! `make strength` wraps the common invocation and refreshes
//! `docs/STRENGTH.md` + `bench/elo.json`.

use std::time::Instant;

use taikyokushogi::elo::{self, EngineConfig, MatchConfig, Wdl};

const DEFAULT_SEED: u64 = 0x5EED_1AD5_0EED_E10C;

struct Args {
    games: usize,
    max_depth: u32,
    movetime: u64,
    maxplies: u32,
    opening: u32,
    threads: usize,
    net: Option<String>,
    json: Option<String>,
    md: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        games: 40,
        max_depth: 3,
        movetime: 300,
        maxplies: 250,
        opening: 8,
        threads: 1,
        net: None,
        json: None,
        md: None,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut it = argv.iter();
    while let Some(k) = it.next() {
        let mut get = || {
            it.next()
                .unwrap_or_else(|| die(&format!("missing value after {k}")))
        };
        match k.as_str() {
            "--games" => a.games = get().parse().unwrap_or_else(|_| die("bad --games")),
            "--max-depth" => a.max_depth = get().parse().unwrap_or_else(|_| die("bad --max-depth")),
            "--movetime" => a.movetime = get().parse().unwrap_or_else(|_| die("bad --movetime")),
            "--maxplies" => a.maxplies = get().parse().unwrap_or_else(|_| die("bad --maxplies")),
            "--open" => a.opening = get().parse().unwrap_or_else(|_| die("bad --open")),
            "--threads" => a.threads = get().parse().unwrap_or_else(|_| die("bad --threads")),
            "--net" => a.net = Some(get().to_string()),
            "--json" => a.json = Some(get().to_string()),
            "--md" => a.md = Some(get().to_string()),
            "--help" | "-h" => {
                println!("{}", std::fs::read_to_string(file!()).unwrap_or_default());
                std::process::exit(0);
            }
            other => die(&format!(
                "unknown argument `{other}` (see the header of examples/elo_report.rs)"
            )),
        }
    }
    if a.max_depth < 2 {
        die("--max-depth must be >= 2 (the ladder compares depth d vs d-1)");
    }
    a
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(2);
}

struct Rung {
    depth: u32,
    wdl: Wdl,
    elo: f64,
    margin: f64,
}

/// One ladder rung: depth `d` vs depth `d - 1`, `games` games with
/// alternating colors and seeded openings. Deterministic.
fn play_rung(args: &Args, d: u32, net: Option<std::sync::Arc<taikyokushogi::Network>>) -> Rung {
    let cfg = MatchConfig {
        games: args.games,
        a: EngineConfig {
            depth: d,
            time_ms: args.movetime,
            net: net.clone(),
            threads: args.threads,
        },
        b: EngineConfig {
            depth: d - 1,
            time_ms: args.movetime,
            net,
            threads: args.threads,
        },
        max_plies: args.maxplies,
        opening_plies: args.opening,
        opening_seed: DEFAULT_SEED ^ u64::from(d).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        on_game: None,
    };
    println!(
        "ladder: depth {} vs depth {} — {} games, movetime {}ms, {} opening plies…",
        d,
        d - 1,
        args.games,
        args.movetime,
        args.opening
    );
    let t = Instant::now();
    let r = elo::run_match(&cfg);
    println!(
        "        W-D-L {}-{}-{}  Elo {:+.1} ± {:.1}  ({:.1}s)",
        r.wdl.wins,
        r.wdl.draws,
        r.wdl.losses,
        r.elo,
        r.margin95,
        t.elapsed().as_secs_f64()
    );
    Rung {
        depth: d,
        wdl: r.wdl,
        elo: r.elo,
        margin: r.margin95,
    }
}

/// Render the ladder as a markdown table (the `--md` payload).
fn markdown(args: &Args, rungs: &[Rung], cumulative: &[(f64, f64)], net_label: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Ladder measured {} ({}) — {} games per rung, movetime {} ms, {} opening plies, \
         {} thread(s), ply cap {}. Evaluator: {}. Depth 1 is the 0-Elo anchor.\n",
        chrono_date(),
        taikyokushogi::VERSION,
        args.games,
        args.movetime,
        args.opening,
        args.threads,
        args.maxplies,
        net_label,
    ));
    out.push_str("\n| Depth | W-D-L vs previous | Pair Elo | Cumulative Elo (95% CI) |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    out.push_str("| 1 | — | — | 0 (anchor) |\n");
    for (i, r) in rungs.iter().enumerate() {
        let (cum, cum_margin) = cumulative[i];
        out.push_str(&format!(
            "| {} | {}-{}-{} | {:+.1} | {:+.1} ± {:.0} |\n",
            r.depth, r.wdl.wins, r.wdl.draws, r.wdl.losses, r.elo, cum, cum_margin
        ));
    }
    let last = cumulative.last().map(|c| c.0).unwrap_or(0.0);
    out.push_str(&format!(
        "\n**Current default depth ({}) ≈ {:+.0} Elo relative to the depth-1 baseline** \
         under these conditions. Small samples: treat the CI as part of the number.\n",
        args.max_depth, last
    ));
    out
}

fn chrono_date() -> String {
    // No chrono dependency: UTC date from the Unix epoch.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn json_report(args: &Args, rungs: &[Rung], cumulative: &[(f64, f64)], net_label: &str) -> String {
    let rungs_json: Vec<String> = rungs
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let (cum, cum_margin) = cumulative[i];
            format!(
                "{{\"depth\":{},\"vs_depth\":{},\"wins\":{},\"draws\":{},\"losses\":{},\"elo\":{:.2},\"margin95\":{:.2},\"cumulative\":{:.2},\"cumulative_margin\":{:.2}}}",
                r.depth,
                r.depth - 1,
                r.wdl.wins,
                r.wdl.draws,
                r.wdl.losses,
                r.elo,
                r.margin,
                cum,
                cum_margin
            )
        })
        .collect();
    format!(
        "{{\"version\":\"{}\",\"date\":\"{}\",\"evaluator\":\"{}\",\"config\":{{\"games\":{},\"max_depth\":{},\"movetime_ms\":{},\"maxplies\":{},\"opening_plies\":{},\"threads\":{}}},\"rungs\":[{}],\"final_elo\":{:.2},\"final_margin\":{:.2}}}\n",
        taikyokushogi::VERSION,
        chrono_date(),
        net_label,
        args.games,
        args.max_depth,
        args.movetime,
        args.maxplies,
        args.opening,
        args.threads,
        rungs_json.join(","),
        cumulative.last().map(|c| c.0).unwrap_or(0.0),
        cumulative.last().map(|c| c.1).unwrap_or(0.0),
    )
}

/// Replace the content between `<!-- elo-report:begin -->` and
/// `<!-- elo-report:end -->` markers in `path` (both markers must exist).
fn update_md(path: &str, payload: &str) {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("cannot read {path}: {e}")));
    const BEGIN: &str = "<!-- elo-report:begin -->";
    const END: &str = "<!-- elo-report:end -->";
    let start = text
        .find(BEGIN)
        .map(|i| i + BEGIN.len())
        .unwrap_or_else(|| die(&format!("{path} has no `{BEGIN}` marker")));
    let end = text[start..]
        .find(END)
        .map(|i| start + i)
        .unwrap_or_else(|| die(&format!("{path} has no `{END}` marker")));
    let updated = format!(
        "{}\n{}\n{}",
        &text[..start],
        payload.trim_end(),
        &text[end..]
    );
    std::fs::write(path, updated).unwrap_or_else(|e| die(&format!("cannot write {path}: {e}")));
    println!("updated {path} (elo-report block)");
}

/// Create the parent directory of a path (e.g. `bench/` for
/// `bench/elo.json`) so a fresh checkout can write the report directly.
fn ensure_parent(path: &str) {
    if let Some(dir) = std::path::Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)
                .unwrap_or_else(|e| die(&format!("cannot create {}: {e}", dir.display())));
        }
    }
}

fn main() {
    let args = parse_args();
    let net = args.net.as_deref().map(|p| {
        std::sync::Arc::new(
            taikyokushogi::Network::load(p)
                .unwrap_or_else(|e| die(&format!("cannot load network `{p}`: {e}"))),
        )
    });
    let net_label = match &args.net {
        Some(p) => format!("NNUE ({p})"),
        None => "hand-crafted".to_string(),
    };
    println!(
        "Elo ladder — v{}, {} games/rung, depths 1..={}, movetime {}ms, evaluator: {}",
        taikyokushogi::VERSION,
        args.games,
        args.max_depth,
        args.movetime,
        net_label
    );

    let mut rungs = Vec::new();
    for d in 2..=args.max_depth {
        rungs.push(play_rung(&args, d, net.clone()));
    }

    // Cumulative rating: depth d = sum of pairwise gains 2..=d; the margins
    // combine as independent standard deviations.
    let mut cumulative = Vec::new();
    let (mut cum, mut var) = (0.0f64, 0.0f64);
    for r in &rungs {
        cum += r.elo;
        var += r.margin * r.margin;
        cumulative.push((cum, var.sqrt()));
    }

    println!("\n=== Elo ladder (anchor: depth 1 = 0) ===");
    for (i, r) in rungs.iter().enumerate() {
        let (cum, m) = cumulative[i];
        println!(
            "depth {:>2}  pair {:+8.1}  cumulative {:+8.1} ± {:.0}   (W-D-L {}-{}-{})",
            r.depth, r.elo, cum, m, r.wdl.wins, r.wdl.draws, r.wdl.losses
        );
    }

    let payload = markdown(&args, &rungs, &cumulative, &net_label);
    print!("\n{payload}");
    if let Some(path) = &args.json {
        ensure_parent(path);
        std::fs::write(path, json_report(&args, &rungs, &cumulative, &net_label))
            .unwrap_or_else(|e| die(&format!("cannot write {path}: {e}")));
        println!("wrote {path}");
    }
    if let Some(path) = &args.md {
        update_md(path, &payload);
    }
}
