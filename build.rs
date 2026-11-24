fn main() {
    let version = std::process::Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .and_then(|o| o.status.success().then_some(o))
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .map(|s| s.strip_prefix('v').unwrap_or(&s).to_string())
        .filter(|s| s.contains('.'))
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=TAIKYOKU_VERSION={}", version);
    println!("cargo:rerun-if-changed=.git/HEAD");
}
