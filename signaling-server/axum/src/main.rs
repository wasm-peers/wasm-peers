use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use warp::Filter;
use wasm_peers_signaling_server::{many_to_many, one_to_many, one_to_one};

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    // let one_to_one_signaling = {
    //     let connections = one_to_one::Connections::default();
    //     let connections = warp::any().map(move || connections.clone());

    //     let sessions = one_to_one::Sessions::default();
    //     let sessions = warp::any().map(move || sessions.clone());

    //     warp::path("one-to-one")
    //         .and(warp::ws())
    //         .and(connections)
    //         .and(sessions)
    //         .map(|ws: warp::ws::Ws, connections, sessions| {
    //             ws.on_upgrade(move |socket| {
    //                 one_to_one::user_connected(socket, connections, sessions)
    //             })
    //         })
    // };

    // let one_to_many_signaling = {
    //     let connections = one_to_many::Connections::default();
    //     let connections = warp::any().map(move || connections.clone());

    //     let sessions = one_to_many::Sessions::default();
    //     let sessions = warp::any().map(move || sessions.clone());

    //     warp::path("one-to-many")
    //         .and(warp::ws())
    //         .and(connections)
    //         .and(sessions)
    //         .map(|ws: warp::ws::Ws, connections, sessions| {
    //             ws.on_upgrade(move |socket| {
    //                 one_to_many::user_connected(socket, connections, sessions)
    //             })
    //         })
    // };

    // let many_to_many_signaling = {
    //     let connections = many_to_many::Connections::default();
    //     let connections = warp::any().map(move || connections.clone());

    //     let sessions = many_to_many::Sessions::default();
    //     let sessions = warp::any().map(move || sessions.clone());

    //     warp::path("many-to-many")
    //         .and(warp::ws())
    //         .and(connections)
    //         .and(sessions)
    //         .map(|ws: warp::ws::Ws, connections, sessions| {
    //             ws.on_upgrade(move |socket| {
    //                 many_to_many::user_connected(socket, connections, sessions)
    //             })
    //         })
    // };

    // build our application with some routes
    let app = Router::new()
        .fallback_service(
            get_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
                .handle_error(|error: std::io::Error| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    )
                }),
        )
        // routes are matched from bottom to top, so we have to put `nest` at the
        // top since it matches all routes
        .route("/ws", get(ws_handler))
        // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run it with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 9001));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
