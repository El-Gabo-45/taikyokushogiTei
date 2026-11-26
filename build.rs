fn git(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|o| o.status.success().then_some(o))
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

fn main() {
    let version = git(&["describe", "--tags", "--always"])
        .map(|s| s.strip_prefix('v').unwrap_or(&s).to_string())
        .filter(|s| s.contains('.'))
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=TAIKYOKU_VERSION={version}");

    // Stockfish-style build provenance, so bug reports can name the exact
    // commit/date/toolchain. All are best-effort: a missing git or a failed
    // command never breaks the build.
    let sha = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=TAIKYOKU_BUILD_SHA={sha}");
    let date = git(&["log", "-1", "--format=%cI"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=TAIKYOKU_BUILD_DATE={date}");
    let rustc = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=TAIKYOKU_RUSTC={rustc}");
    println!(
        "cargo:rustc-env=TAIKYOKU_BUILD_TARGET={}",
        std::env::var("TARGET").unwrap_or_else(|_| "unknown".into())
    );

    println!("cargo:rerun-if-changed=.git/HEAD");
}
