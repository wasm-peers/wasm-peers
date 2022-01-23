use crate::document::DocumentQuery;
use rusty_games_library::get_random_session_id;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::Route;

pub(crate) enum HomeMsg {
    UpdateInput(String),
}

pub(crate) struct Home {
    input: String,
}

impl Component for Home {
    type Message = HomeMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            input: String::new(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Self::Message::UpdateInput(input) => {
                self.input += &input;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let history = ctx.link().history().unwrap();
        let start_as_host = {
            let history = history.clone();
            Callback::once(move |_| {
                history
                    .push_with_query(
                        Route::Document,
                        DocumentQuery::new(get_random_session_id().into_inner()),
                    )
                    .unwrap();
            })
        };
        let update_input = ctx
            .link()
            .callback(|e: InputEvent| HomeMsg::UpdateInput(e.data().unwrap_or_else(String::new)));
        let join_existing = {
            let session_id = self.input.clone();
            Callback::once(move |_| {
                history
                    .push_with_query(Route::Document, DocumentQuery::new(session_id))
                    .unwrap();
            })
        };
        html! {
            <div>
                <div style="border:1px solid black; text-align:center">
                    <p>{ "Live Document is a shared document application akin to Google Docs." }</p>
                    <p>{ "Create new document, or join existing one." }</p>
                    <p>{ "Document lives as long as somebody is in session." }</p>
                    <p>{ "Persistent storage coming soon!" }</p>
                </div>
                <div style="text-align:center">
                    <p>{ "Start as host" }</p>
                    <button onclick={ start_as_host }>{ "Start as host" }</button>
                </div>
                <div style="text-align:center">
                    <p>{ "Join existing document" }</p>
                    <input
                        value={ self.input.clone() }
                        oninput={ update_input }
                    />
                    <button onclick={ join_existing }>{ "Join" }</button>
                </div>
            </div>
        }
    }
}
