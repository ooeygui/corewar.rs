use serde::Serialize;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

pub(crate) fn to_js_value<T>(value: &T) -> JsValue
where
    T: Serialize,
{
    serde_wasm_bindgen::to_value(value).unwrap_or_else(|err| JsValue::from_str(&err.to_string()))
}

pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn init() {
    set_panic_hook();
}
