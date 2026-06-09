use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use wasm_bindgen::{closure::Closure, prelude::*, JsCast};
use web_sys::{Document, Event, HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent};

use crate::{
    app::{InputEvent, KeyCode, MouseButton},
    App, ColorPalette, Renderer, StateSynchronizer,
};

thread_local! {
    static WEB_RUNTIME: RefCell<Option<Rc<WebRuntime>>> = const { RefCell::new(None) };
}

struct WebRuntime {
    window: web_sys::Window,
    canvas: &'static HtmlCanvasElement,
    app: RefCell<App<'static>>,
    synchronizer: RefCell<StateSynchronizer>,
    last_frame_ms: Cell<Option<f64>>,
}

#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    if WEB_RUNTIME.with(|runtime| runtime.borrow().is_some()) {
        return Ok(());
    }

    let window = web_sys::window().ok_or_else(|| js_error("window is not available"))?;
    let document = window
        .document()
        .ok_or_else(|| js_error("document is not available"))?;
    let canvas = Box::leak(Box::new(get_canvas(&document)?));

    let (width, height) = viewport_size(&window)?;
    canvas.set_width(width);
    canvas.set_height(height);

    let renderer = Renderer::new(canvas)
        .await
        .map_err(|err| js_error(err.to_string()))?;
    let mut app = App::new(
        renderer,
        ColorPalette::generate(256),
        width as f32,
        height as f32,
    );
    app.handle_event(InputEvent::Resize {
        width: width as f32,
        height: height as f32,
    });

    let (server_url, instance_id) = query_config(&window)?;
    let synchronizer = StateSynchronizer::connect(&server_url, instance_id)
        .await
        .map_err(|err| js_error(err.to_string()))?;

    let runtime = Rc::new(WebRuntime {
        window,
        canvas,
        app: RefCell::new(app),
        synchronizer: RefCell::new(synchronizer),
        last_frame_ms: Cell::new(None),
    });

    install_event_listeners(&runtime)?;
    schedule_next_frame(Rc::clone(&runtime))?;
    WEB_RUNTIME.with(|slot| *slot.borrow_mut() = Some(runtime));

    Ok(())
}

impl WebRuntime {
    fn resize_canvas(&self) -> Result<(), JsValue> {
        let (width, height) = viewport_size(&self.window)?;
        self.canvas.set_width(width);
        self.canvas.set_height(height);

        let mut app = self.app.borrow_mut();
        app.renderer.resize_surface(width, height);
        app.handle_event(InputEvent::Resize {
            width: width as f32,
            height: height as f32,
        });
        Ok(())
    }

    fn dispatch_input(&self, event: InputEvent) {
        self.app.borrow_mut().handle_event(event);
    }

    async fn frame(self: Rc<Self>, timestamp_ms: f64) -> Result<(), JsValue> {
        let dt = self
            .last_frame_ms
            .replace(Some(timestamp_ms))
            .map(|previous| ((timestamp_ms - previous) / 1_000.0).max(0.0) as f32)
            .unwrap_or_default();

        let mut app = self.app.borrow_mut();
        let mut synchronizer = self.synchronizer.borrow_mut();
        synchronizer
            .synchronize_app(&mut app)
            .await
            .map_err(|err| js_error(err.to_string()))?;
        app.update(dt);
        app.render();
        Ok(())
    }
}

fn install_event_listeners(runtime: &Rc<WebRuntime>) -> Result<(), JsValue> {
    let canvas = runtime.canvas.clone();
    let window = runtime.window.clone();

    {
        let runtime = Rc::clone(runtime);
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            runtime.dispatch_input(InputEvent::MouseMove {
                position: mouse_position(&event),
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let runtime = Rc::clone(runtime);
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            event.prevent_default();
            runtime.dispatch_input(InputEvent::MouseDown {
                button: mouse_button(event.button()),
                position: mouse_position(&event),
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let runtime = Rc::clone(runtime);
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            runtime.dispatch_input(InputEvent::MouseUp {
                button: mouse_button(event.button()),
                position: mouse_position(&event),
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let runtime = Rc::clone(runtime);
        let closure = Closure::wrap(Box::new(move |event: WheelEvent| {
            event.prevent_default();
            runtime.dispatch_input(InputEvent::Scroll {
                delta_y: (event.delta_y() as f32 / 100.0).clamp(-4.0, 4.0),
                position: glam::Vec2::new(event.offset_x() as f32, event.offset_y() as f32),
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let runtime = Rc::clone(runtime);
        let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            if let Some(key) = map_key(&event.key()) {
                event.prevent_default();
                runtime.dispatch_input(InputEvent::KeyPress { key });
            }
        }) as Box<dyn FnMut(_)>);
        window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let runtime = Rc::clone(runtime);
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            if let Err(err) = runtime.resize_canvas() {
                web_sys::console::error_1(&err);
            }
        }) as Box<dyn FnMut(_)>);
        window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}

fn schedule_next_frame(runtime: Rc<WebRuntime>) -> Result<(), JsValue> {
    let window = runtime.window.clone();
    let callback = Closure::once_into_js(move |timestamp_ms: f64| {
        let runtime = Rc::clone(&runtime);
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(err) = Rc::clone(&runtime).frame(timestamp_ms).await {
                web_sys::console::error_1(&err);
            }
            if let Err(err) = schedule_next_frame(runtime) {
                web_sys::console::error_1(&err);
            }
        });
    });

    window.request_animation_frame(callback.unchecked_ref())?;
    Ok(())
}

fn get_canvas(document: &Document) -> Result<HtmlCanvasElement, JsValue> {
    document
        .get_element_by_id("corewar-canvas")
        .ok_or_else(|| js_error("missing #corewar-canvas element"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| js_error("#corewar-canvas is not a canvas element"))
}

fn viewport_size(window: &web_sys::Window) -> Result<(u32, u32), JsValue> {
    let width = window
        .inner_width()?
        .as_f64()
        .ok_or_else(|| js_error("window width is not numeric"))?
        .max(1.0)
        .round() as u32;
    let height = window
        .inner_height()?
        .as_f64()
        .ok_or_else(|| js_error("window height is not numeric"))?
        .max(1.0)
        .round() as u32;
    Ok((width, height))
}

fn query_config(window: &web_sys::Window) -> Result<(String, String), JsValue> {
    let search = window.location().search()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search)?;
    let server_url = params
        .get("server")
        .or_else(|| params.get("ws"))
        .unwrap_or_else(|| "ws://localhost:9000".to_string());
    let instance_id = params
        .get("instance")
        .or_else(|| params.get("instance_id"))
        .unwrap_or_else(|| "arena-1".to_string());
    Ok((server_url, instance_id))
}

fn mouse_position(event: &MouseEvent) -> glam::Vec2 {
    glam::Vec2::new(event.offset_x() as f32, event.offset_y() as f32)
}

fn mouse_button(button: i16) -> MouseButton {
    match button {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        other => MouseButton::Other(other as u16),
    }
}

fn map_key(key: &str) -> Option<KeyCode> {
    match key {
        " " => Some(KeyCode::Space),
        "+" => Some(KeyCode::Plus),
        "=" => Some(KeyCode::Equals),
        "-" => Some(KeyCode::Minus),
        _ => {
            let mut chars = key.chars();
            let ch = chars.next()?;
            if chars.next().is_some() {
                return None;
            }

            Some(match ch.to_ascii_lowercase() {
                'r' => KeyCode::R,
                'f' => KeyCode::F,
                other => KeyCode::Character(other),
            })
        }
    }
}

fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}
