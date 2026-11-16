//! Calling the app and hearing from it, through the bridge Tauri puts on the page
//! (`withGlobalTauri`). Without it (a plain browser, for design work) nothing is there to answer.

use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["__TAURI__", "core"], js_name = invoke)]
    async fn invoke_raw(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["__TAURI__", "event"], js_name = listen)]
    fn listen_raw(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> js_sys::Promise;
}

/// Is the page running inside the app?
pub fn present() -> bool {
    js_sys::Reflect::get(&js_sys::global(), &"__TAURI__".into()).is_ok_and(|v| !v.is_undefined())
}

fn to_js<T: Serialize>(v: &T) -> JsValue {
    js_sys::JSON::parse(&serde_json::to_string(v).unwrap()).unwrap()
}

fn from_js<T: DeserializeOwned>(v: &JsValue) -> Result<T, String> {
    if v.is_undefined() {
        return serde_json::from_str("null").map_err(|e| e.to_string());
    }
    let text: String = js_sys::JSON::stringify(v).map_err(|e| format!("{e:?}"))?.into();
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn message(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| {
        js_sys::JSON::stringify(&e).ok().and_then(|s| s.as_string()).unwrap_or_else(|| format!("{e:?}"))
    })
}

/// Calls a command; the error is the app's message, as the user would read it.
pub async fn invoke<T: DeserializeOwned>(cmd: &str, args: impl Serialize) -> Result<T, String> {
    if !present() {
        return Err("not in the app".into());
    }
    let out = invoke_raw(cmd, to_js(&args)).await.map_err(message)?;
    from_js(&out)
}

/// The same, for commands whose answer doesn't matter.
pub async fn call(cmd: &str, args: impl Serialize) -> Result<(), String> {
    invoke::<serde_json::Value>(cmd, args).await.map(|_| ())
}

/// No arguments.
pub fn none() -> serde_json::Value {
    serde_json::json!({})
}

/// Runs `f` with each payload of `event` sent to this window.
pub fn listen<T: DeserializeOwned + 'static>(event: &str, mut f: impl FnMut(T) + 'static) {
    if !present() {
        return;
    }
    let cb = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
        let payload = js_sys::Reflect::get(&e, &"payload".into()).unwrap_or(JsValue::NULL);
        match from_js::<T>(&payload) {
            Ok(p) => f(p),
            Err(err) => web_sys::console::error_1(&format!("bad payload: {err}").into()),
        }
    });
    let _ = listen_raw(event, &cb);
    cb.forget();
}

/// Starts an async task.
pub fn spawn(f: impl std::future::Future<Output = ()> + 'static) {
    wasm_bindgen_futures::spawn_local(f);
}
