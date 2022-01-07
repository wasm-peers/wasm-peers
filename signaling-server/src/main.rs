use log::LevelFilter;
use simplelog::{Config, TermLogger, TerminalMode};
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

use warp::Filter;

use rusty_games_signaling_server::server::{user_connected, Connections, Sessions};

#[tokio::main]
async fn main() {
    TermLogger::init(LevelFilter::Debug, Config::default(), TerminalMode::Mixed).unwrap();

    let connections = Connections::default();
    let connections = warp::any().map(move || connections.clone());

    let sessions = Sessions::default();
    let sessions = warp::any().map(move || sessions.clone());

    let signaling = warp::path("ws")
        .and(warp::ws())
        .and(connections)
        .and(sessions)
        .map(|ws: warp::ws::Ws, connections, sessions| {
            ws.on_upgrade(move |socket| user_connected(socket, connections, sessions))
        });

    let address = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:9001".to_string());
    let address = SocketAddr::from_str(&address).expect("invalid ip address provided");

    warp::serve(signaling).run(address).await;
}
