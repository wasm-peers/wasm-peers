use std::env;

use warp::Filter;

use string_metric_server::server::{user_connected, Connections};

#[tokio::main]
async fn main() {
    let connections = Connections::default();
    let connections = warp::any().map(move || connections.clone());

    let string_metric =
        warp::path("ws")
            .and(warp::ws())
            .and(connections)
            .map(|ws: warp::ws::Ws, connections| {
                ws.on_upgrade(move |socket| user_connected(socket, connections))
            });

    let port = match env::args().nth(1) {
        Some(s) => s.parse::<u16>().unwrap(),
        None => 8080,
    };

    warp::serve(string_metric).run(([127, 0, 0, 1], port)).await;
}
