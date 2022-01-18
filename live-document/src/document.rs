use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{console, HtmlTextAreaElement, UrlSearchParams};
use yew::{html, Component, Context, Html};
use serde::{Serialize, Deserialize};

use rusty_games_library::{ConnectionType, NetworkManager, SessionId};

pub(crate) enum DocumentMsg {
    UpdateValue,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentQuery {
    pub session_id: String,
}

impl DocumentQuery {
    pub(crate) fn new(session_id: String) -> Self {
        DocumentQuery { session_id }
    }
}

pub(crate) struct Document {
    session_id: SessionId,
    network_manager: NetworkManager,
    is_ready: Rc<RefCell<bool>>,
}

fn get_queries() -> UrlSearchParams {
    let search = web_sys::window().unwrap().location().search().unwrap();
    UrlSearchParams::new_with_str(&search).unwrap()
}

impl Component for Document {
    type Message = DocumentMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let session_id = SessionId::new(get_queries().get("session_id").unwrap());
        let mut network_manager = NetworkManager::new(
            env!("WS_IP_PORT"),
            session_id.clone(),
            ConnectionType::Stun,
        )
        .unwrap();

        let is_ready = Rc::new(RefCell::new(false));
        let on_open_callback = {
            let is_ready = is_ready.clone();
            move || {
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("document-textarea")
                    .expect("could not find textarea element by id")
                    .dyn_ref::<HtmlTextAreaElement>()
                    .expect("element is not a textarea")
                    .set_disabled(false);
                *is_ready.borrow_mut() = true;
            }
        };
        let on_message_callback = {
            move |message: String| {
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("document-textarea")
                    .expect("could not find textarea element by id")
                    .dyn_ref::<HtmlTextAreaElement>()
                    .expect("element is not a textarea")
                    .set_value(message.strip_prefix('x').unwrap());
            }
        };
        network_manager
            .start(on_open_callback, on_message_callback)
            .expect("couldn't start network manager");
        Self {
            is_ready,
            session_id,
            network_manager,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Self::Message::UpdateValue => {
                let textarea_value = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("document-textarea")
                    .expect("could not find textarea element by id")
                    .dyn_ref::<HtmlTextAreaElement>()
                    .expect("element is not a textarea")
                    .value();
                self.network_manager
                    .send_message(&format!("x{}", textarea_value))
                    .unwrap_or_else(|_| {
                        console::error_1(&"couldn't send message yet!".to_string().into())
                    });
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let oninput = ctx.link().callback(|_| Self::Message::UpdateValue);
        let disabled = !*self.is_ready.borrow();
        let placeholder = if disabled {
            "This is a live document shared with a different user.\nYou will be allowed to write once other user connects."
        } else {
            "This is a live document shared with a different user.\nWhat you both write will be visible to the other."
        };
        html! {
            <main>
                <p> { "Session id: " } { &self.session_id.inner } </p>
                <textarea id={ "document-textarea" } { disabled } { placeholder } { oninput }/>
            </main>
        }
    }
}
