use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use wasm_peers_signaling_server::router::{self, ServerState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    let server_state = ServerState::default();
    let app = router::create(server_state);

    let address = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:9001".to_string());
    let address = SocketAddr::from_str(&address)?;

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
