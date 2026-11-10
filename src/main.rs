//! Taikyoku Shogi HTTP Server + Web GUI.
//!
//! Serves the REST API (implemented in `taikyokushogi::server`) and the
//! static files of the web frontend.
//!
//! Run with: `cargo run --features server --bin taikyokushogi-server`
//! Bind address: `TAIKYOKU_BIND` (default `127.0.0.1:8000`).
//! Frontend dir: `TAIKYOKU_DIST` (default `web/dist`).
//! NNUE network: `TAIKYOKU_NET` (e.g. `nets/experimental-v1.tkn` or
//! `nets/experimental-v1.tkn.zst`; `none` to disable). The engine panel reports
//! `Backend: nnue` when one is loaded.

#[tokio::main]
async fn main() {
    taikyokushogi::server::run().await;
}
