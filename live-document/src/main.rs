use rusty_games_library::{set_panic_hook, SessionId};
use yew::{html, Component, Context, Html};
use yew_router::prelude::*;

use crate::document::Document;
use crate::home::Home;

mod document;
mod home;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/document/:session_id/:is_host")]
    Document {
        session_id: SessionId,
        is_host: bool,
    },
}

struct App {}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
            <main>
            <Switch<Route> render={Switch::render(switch)} />
            </main>
            </BrowserRouter>
        }
    }
}

fn switch(routes: &Route) -> Html {
    match routes.clone() {
        Route::Home => {
            html! { <Home /> }
        }
        Route::Document {
            session_id,
            is_host,
        } => {
            html! { <Document {session_id} {is_host} /> }
        }
    }
}

fn main() {
    set_panic_hook();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    yew::start_app::<App>();
}
