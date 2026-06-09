use std::{
    cell::{Cell, RefCell},
    mem,
    rc::Rc,
};

use corewar_protocol::{ClientMessage, ServerMessage};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{BinaryType, Event, MessageEvent, WebSocket};

use crate::NetworkError;

const INITIAL_BACKOFF_MS: i32 = 250;
const MAX_BACKOFF_MS: i32 = 5_000;

/// WASM WebSocket client for the visualization frontend.
pub struct NetworkClient {
    shared: Rc<Shared>,
}

struct Shared {
    url: String,
    websocket: RefCell<Option<WebSocket>>,
    callbacks: RefCell<Option<Callbacks>>,
    messages: RefCell<Vec<ServerMessage>>,
    pending_outbound: RefCell<Vec<ClientMessage>>,
    connected: Cell<bool>,
    reconnecting: Cell<bool>,
    closed: Cell<bool>,
}

#[allow(dead_code)]
struct Callbacks {
    onopen: Closure<dyn FnMut(Event)>,
    onmessage: Closure<dyn FnMut(MessageEvent)>,
    onclose: Closure<dyn FnMut(Event)>,
    onerror: Closure<dyn FnMut(Event)>,
}

impl NetworkClient {
    pub async fn connect(url: &str) -> Result<Self, NetworkError> {
        let shared = Rc::new(Shared {
            url: url.to_owned(),
            websocket: RefCell::new(None),
            callbacks: RefCell::new(None),
            messages: RefCell::new(Vec::new()),
            pending_outbound: RefCell::new(Vec::new()),
            connected: Cell::new(false),
            reconnecting: Cell::new(false),
            closed: Cell::new(false),
        });

        Shared::open_socket(Rc::clone(&shared))?;

        Ok(Self { shared })
    }

    pub async fn send(&mut self, msg: ClientMessage) -> Result<(), NetworkError> {
        if self.shared.connected.get() {
            if let Err(err) = self.shared.send_now(&msg) {
                self.shared.pending_outbound.borrow_mut().push(msg);
                self.shared.connected.set(false);
                Shared::schedule_reconnect(Rc::clone(&self.shared));
                return Err(err);
            }
            return Ok(());
        }

        self.shared.pending_outbound.borrow_mut().push(msg);
        Shared::schedule_reconnect(Rc::clone(&self.shared));
        Ok(())
    }

    pub fn poll_messages(&mut self) -> Vec<ServerMessage> {
        mem::take(&mut *self.shared.messages.borrow_mut())
    }

    pub fn is_connected(&self) -> bool {
        self.shared.connected.get()
    }
}

impl Drop for NetworkClient {
    fn drop(&mut self) {
        self.shared.closed.set(true);
        self.shared.connected.set(false);
        if let Some(websocket) = self.shared.websocket.borrow_mut().take() {
            websocket.set_onopen(None);
            websocket.set_onmessage(None);
            websocket.set_onclose(None);
            websocket.set_onerror(None);
            let _ = websocket.close();
        }
        self.shared.callbacks.borrow_mut().take();
    }
}

impl Shared {
    fn open_socket(shared: Rc<Self>) -> Result<(), NetworkError> {
        let websocket = WebSocket::new(&shared.url)
            .map_err(|err| NetworkError::Connect(js_error_to_string(err)))?;
        websocket.set_binary_type(BinaryType::Arraybuffer);

        let onopen_shared = Rc::clone(&shared);
        let onopen = Closure::wrap(Box::new(move |_event: Event| {
            onopen_shared.connected.set(true);
            onopen_shared.reconnecting.set(false);
            if let Err(err) = onopen_shared.flush_pending() {
                onopen_shared.connected.set(false);
                Shared::schedule_reconnect(Rc::clone(&onopen_shared));
                tracing::warn!(error = %err, "failed to flush pending websocket messages");
            }
        }) as Box<dyn FnMut(_)>);

        let onmessage_shared = Rc::clone(&shared);
        let onmessage = Closure::wrap(Box::new(
            move |event: MessageEvent| match decode_message_event(&event) {
                Ok(Some(message)) => onmessage_shared.messages.borrow_mut().push(message),
                Ok(None) => {}
                Err(err) => tracing::warn!(error = %err, "failed to decode websocket message"),
            },
        ) as Box<dyn FnMut(_)>);

        let onclose_shared = Rc::clone(&shared);
        let onclose = Closure::wrap(Box::new(move |_event: Event| {
            onclose_shared.connected.set(false);
            if !onclose_shared.closed.get() {
                Shared::schedule_reconnect(Rc::clone(&onclose_shared));
            }
        }) as Box<dyn FnMut(_)>);

        let onerror_shared = Rc::clone(&shared);
        let onerror = Closure::wrap(Box::new(move |_event: Event| {
            onerror_shared.connected.set(false);
            if !onerror_shared.closed.get() {
                Shared::schedule_reconnect(Rc::clone(&onerror_shared));
            }
        }) as Box<dyn FnMut(_)>);

        websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        websocket.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        *shared.websocket.borrow_mut() = Some(websocket);
        *shared.callbacks.borrow_mut() = Some(Callbacks {
            onopen,
            onmessage,
            onclose,
            onerror,
        });

        Ok(())
    }

    fn schedule_reconnect(shared: Rc<Self>) {
        if shared.closed.get() || shared.reconnecting.replace(true) {
            return;
        }

        spawn_local(async move {
            let mut delay_ms = INITIAL_BACKOFF_MS;
            loop {
                if shared.closed.get() || shared.connected.get() {
                    break;
                }

                sleep_ms(delay_ms).await;

                if shared.closed.get() || shared.connected.get() {
                    break;
                }

                if let Some(existing) = shared.websocket.borrow_mut().take() {
                    existing.set_onopen(None);
                    existing.set_onmessage(None);
                    existing.set_onclose(None);
                    existing.set_onerror(None);
                    let _ = existing.close();
                }
                shared.callbacks.borrow_mut().take();

                match Shared::open_socket(Rc::clone(&shared)) {
                    Ok(()) => break,
                    Err(err) => {
                        tracing::warn!(error = %err, delay_ms, "websocket reconnect attempt failed");
                        delay_ms = (delay_ms.saturating_mul(2)).min(MAX_BACKOFF_MS);
                    }
                }
            }

            shared.reconnecting.set(false);
        });
    }

    fn flush_pending(&self) -> Result<(), NetworkError> {
        let pending = mem::take(&mut *self.pending_outbound.borrow_mut());
        for message in pending {
            self.send_now(&message)?;
        }
        Ok(())
    }

    fn send_now(&self, msg: &ClientMessage) -> Result<(), NetworkError> {
        let payload = serde_json::to_string(msg)?;
        let websocket = self.websocket.borrow();
        let websocket = websocket.as_ref().ok_or(NetworkError::ChannelClosed)?;
        websocket
            .send_with_str(&payload)
            .map_err(|err| NetworkError::Send(js_error_to_string(err)))
    }
}

async fn sleep_ms(timeout_ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(window) = web_sys::window() {
            let callback = Closure::once_into_js(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                timeout_ms,
            );
        } else {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });

    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

fn decode_message_event(event: &MessageEvent) -> Result<Option<ServerMessage>, NetworkError> {
    let data = event.data();

    if let Some(text) = data.as_string() {
        return serde_json::from_str(&text)
            .map(Some)
            .map_err(NetworkError::DecodeJson);
    }

    if let Ok(buffer) = data.dyn_into::<ArrayBuffer>() {
        let bytes = Uint8Array::new(&buffer).to_vec();
        return rmp_serde::from_slice(&bytes)
            .map(Some)
            .map_err(NetworkError::DecodeMessagePack);
    }

    Ok(None)
}

fn js_error_to_string(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
