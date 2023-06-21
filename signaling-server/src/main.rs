use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use wasm_peers_signaling_server::router::{self, ServerState};

#[shuttle_runtime::main]
async fn signaling_server() -> shuttle_axum::ShuttleAxum {
    let _ = TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );

    let server_state = ServerState::default();
    let router = router::create(server_state);

    Ok(router.into())
}
