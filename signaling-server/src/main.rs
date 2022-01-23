use log::LevelFilter;
use simplelog::{Config, TermLogger, TerminalMode};
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

use warp::Filter;

use rusty_games_signaling_server::{many_to_many, one_to_many, one_to_one};

#[tokio::main]
async fn main() {
    TermLogger::init(LevelFilter::Debug, Config::default(), TerminalMode::Mixed).unwrap();

    let one_to_one_signaling = {
        let connections = one_to_one::Connections::default();
        let connections = warp::any().map(move || connections.clone());

        let sessions = one_to_one::Sessions::default();
        let sessions = warp::any().map(move || sessions.clone());

        warp::path("one-to-one")
            .and(warp::ws())
            .and(connections)
            .and(sessions)
            .map(|ws: warp::ws::Ws, connections, sessions| {
                ws.on_upgrade(move |socket| {
                    one_to_one::user_connected(socket, connections, sessions)
                })
            })
    };

    let one_to_many_signaling = {
        let connections = one_to_many::Connections::default();
        let connections = warp::any().map(move || connections.clone());

        let sessions = one_to_many::Sessions::default();
        let sessions = warp::any().map(move || sessions.clone());

        warp::path("one-to-many")
            .and(warp::ws())
            .and(connections)
            .and(sessions)
            .map(|ws: warp::ws::Ws, connections, sessions| {
                ws.on_upgrade(move |socket| {
                    one_to_many::user_connected(socket, connections, sessions)
                })
            })
    };

    let many_to_many_signaling = {
        let connections = many_to_many::Connections::default();
        let connections = warp::any().map(move || connections.clone());

        let sessions = many_to_many::Sessions::default();
        let sessions = warp::any().map(move || sessions.clone());

        warp::path("many-to-many")
            .and(warp::ws())
            .and(connections)
            .and(sessions)
            .map(|ws: warp::ws::Ws, connections, sessions| {
                ws.on_upgrade(move |socket| {
                    many_to_many::user_connected(socket, connections, sessions)
                })
            })
    };

    let routes = one_to_one_signaling
        .or(one_to_many_signaling)
        .or(many_to_many_signaling);

    let address = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:9001".to_string());
    let address = SocketAddr::from_str(&address).expect("invalid IP address provided");

    warp::serve(routes).run(address).await;
}
