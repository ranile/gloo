use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsValue, UnwrapThrowExt};

/// A constant to prevent state collision.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum HistoryStateKind {
    #[serde(rename = "gloo_history_state")]
    Gloo,
}

/// The state used by browser history to store history id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HistoryState {
    id: u32,
    kind: HistoryStateKind,
}

impl HistoryState {
    pub fn id(&self) -> u32 {
        self.id
    }
}

/// Compose a JS object `{ id, kind, state? }` to pass to `history.pushState()`.
pub(crate) fn compose_state(id: u32, user_state: Option<JsValue>) -> JsValue {
    let history_state = HistoryState {
        id,
        kind: HistoryStateKind::Gloo,
    };

    let js = serde_wasm_bindgen::to_value(&history_state)
        .expect_throw("failed to serialize history state");

    if let Some(state) = user_state {
        let key = JsValue::from_str("state");
        js_sys::Reflect::set(&js, &key, &state).expect_throw("failed to set state");
    }

    js
}

/// Extract `(id, user_state)` from a raw `history.state` JsValue.
///
/// Returns `(None, None)` if the value is not a gloo-managed state object.
pub(crate) fn extract_state(raw: JsValue) -> (Option<u32>, Option<JsValue>) {
    let history_state = match serde_wasm_bindgen::from_value::<HistoryState>(raw.clone()) {
        Ok(hs) => hs,
        Err(_) => return (None, None),
    };

    let id = Some(history_state.id());

    let key = JsValue::from_str("state");
    let user_state = js_sys::Reflect::get(&raw, &key).ok().and_then(|v| {
        if v.is_undefined() || v.is_null() {
            None
        } else {
            Some(v)
        }
    });

    (id, user_state)
}
