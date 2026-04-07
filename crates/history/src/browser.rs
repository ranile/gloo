use std::{borrow::Cow, cell::RefCell, fmt, rc::Rc};

use gloo_events::EventListener;
use gloo_utils::window;
use serde::Serialize;
use wasm_bindgen::{JsValue, UnwrapThrowExt};
use web_sys::Url;

use crate::history::History;
use crate::listener::HistoryListener;
use crate::location::Location;
use crate::state::{compose_state, extract_state};
use crate::utils::{WeakCallback, get_id};
#[cfg(feature = "query")]
use crate::{error::HistoryResult, query::ToQuery};

/// A [`History`] that is implemented with [`web_sys::History`] that provides native browser
/// history and state access.
#[derive(Clone)]
pub struct BrowserHistory {
    inner: web_sys::History,
    callbacks: Rc<RefCell<Vec<WeakCallback>>>,
}

impl fmt::Debug for BrowserHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrowserHistory").finish_non_exhaustive()
    }
}

impl PartialEq for BrowserHistory {
    fn eq(&self, _rhs: &Self) -> bool {
        // All browser histories are created equal.
        true
    }
}

impl History for BrowserHistory {
    fn len(&self) -> usize {
        self.inner.length().expect_throw("failed to get length.") as usize
    }

    fn go(&self, delta: isize) {
        self.inner
            .go_with_delta(delta as i32)
            .expect_throw("failed to call go.")
    }

    fn push<'a>(&self, route: impl Into<Cow<'a, str>>) {
        let url = route.into();
        let state = Self::create_state(None);
        self.inner
            .push_state_with_url(&state, "", Some(&url))
            .expect_throw("failed to push state.");

        self.notify_callbacks();
    }

    fn replace<'a>(&self, route: impl Into<Cow<'a, str>>) {
        let url = route.into();
        let state = Self::create_state(None);
        self.inner
            .replace_state_with_url(&state, "", Some(&url))
            .expect_throw("failed to replace history.");

        self.notify_callbacks();
    }

    fn push_with_state<'a, T>(&self, route: impl Into<Cow<'a, str>>, state: T)
    where
        T: Serialize + 'static,
    {
        let url = route.into();
        let js_state =
            serde_wasm_bindgen::to_value(&state).expect_throw("failed to serialize state.");
        let history_state = Self::create_state(Some(js_state));

        self.inner
            .push_state_with_url(&history_state, "", Some(&url))
            .expect_throw("failed to push state.");

        self.notify_callbacks();
    }

    fn replace_with_state<'a, T>(&self, route: impl Into<Cow<'a, str>>, state: T)
    where
        T: Serialize + 'static,
    {
        let url = route.into();
        let js_state =
            serde_wasm_bindgen::to_value(&state).expect_throw("failed to serialize state.");
        let history_state = Self::create_state(Some(js_state));

        self.inner
            .replace_state_with_url(&history_state, "", Some(&url))
            .expect_throw("failed to replace state.");

        self.notify_callbacks();
    }

    #[cfg(feature = "query")]
    fn push_with_query<'a, Q>(
        &self,
        route: impl Into<Cow<'a, str>>,
        query: Q,
    ) -> HistoryResult<(), Q::Error>
    where
        Q: ToQuery,
    {
        let route = route.into();
        let query = query.to_query()?;

        let url = Self::combine_url(&route, &query);
        let state = Self::create_state(None);

        self.inner
            .push_state_with_url(&state, "", Some(&url))
            .expect_throw("failed to push history.");

        self.notify_callbacks();
        Ok(())
    }

    #[cfg(feature = "query")]
    fn replace_with_query<'a, Q>(
        &self,
        route: impl Into<Cow<'a, str>>,
        query: Q,
    ) -> HistoryResult<(), Q::Error>
    where
        Q: ToQuery,
    {
        let route = route.into();
        let query = query.to_query()?;

        let url = Self::combine_url(&route, &query);
        let state = Self::create_state(None);

        self.inner
            .replace_state_with_url(&state, "", Some(&url))
            .expect_throw("failed to replace history.");

        self.notify_callbacks();
        Ok(())
    }

    #[cfg(feature = "query")]
    fn push_with_query_and_state<'a, Q, T>(
        &self,
        route: impl Into<Cow<'a, str>>,
        query: Q,
        state: T,
    ) -> HistoryResult<(), Q::Error>
    where
        Q: ToQuery,
        T: Serialize + 'static,
    {
        let js_state =
            serde_wasm_bindgen::to_value(&state).expect_throw("failed to serialize state.");
        let history_state = Self::create_state(Some(js_state));

        let route = route.into();
        let query = query.to_query()?;

        let url = Self::combine_url(&route, &query);

        self.inner
            .push_state_with_url(&history_state, "", Some(&url))
            .expect_throw("failed to push history.");

        self.notify_callbacks();
        Ok(())
    }

    #[cfg(feature = "query")]
    fn replace_with_query_and_state<'a, Q, T>(
        &self,
        route: impl Into<Cow<'a, str>>,
        query: Q,
        state: T,
    ) -> HistoryResult<(), Q::Error>
    where
        Q: ToQuery,
        T: Serialize + 'static,
    {
        let js_state =
            serde_wasm_bindgen::to_value(&state).expect_throw("failed to serialize state.");
        let history_state = Self::create_state(Some(js_state));

        let route = route.into();
        let query = query.to_query()?;

        let url = Self::combine_url(&route, &query);

        self.inner
            .replace_state_with_url(&history_state, "", Some(&url))
            .expect_throw("failed to replace history.");

        self.notify_callbacks();
        Ok(())
    }

    fn listen<CB>(&self, callback: CB) -> HistoryListener
    where
        CB: Fn() + 'static,
    {
        // Callbacks do not receive a copy of [`History`] to prevent reference cycle.
        let cb = Rc::new(callback) as Rc<dyn Fn()>;

        self.callbacks.borrow_mut().push(Rc::downgrade(&cb));

        HistoryListener { _listener: cb }
    }

    fn location(&self) -> Location {
        let loc = window().location();

        let raw_state = self.inner.state().expect_throw("failed to get state");
        let (id, user_state) = extract_state(raw_state);

        Location {
            path: loc.pathname().expect_throw("failed to get pathname").into(),
            query_str: loc
                .search()
                .expect_throw("failed to get location query.")
                .into(),
            hash: loc
                .hash()
                .expect_throw("failed to get location hash.")
                .into(),
            state: user_state,
            id,
        }
    }
}

impl Default for BrowserHistory {
    fn default() -> Self {
        // We create browser history only once.
        thread_local! {
            static BROWSER_HISTORY: (BrowserHistory, EventListener) = {
                let window = window();

                let inner = window
                    .history()
                    .expect_throw("Failed to create browser history. Are you using a browser?");
                let callbacks = Rc::default();

                let history = BrowserHistory {
                    inner,
                    callbacks,
                };

                let listener = {
                    let history = history.clone();

                    // Listens to popstate.
                    EventListener::new(&window, "popstate", move |_| {
                        history.notify_callbacks();
                    })
                };

                (history, listener)
            };
        }

        BROWSER_HISTORY.with(|(history, _)| history.clone())
    }
}

impl BrowserHistory {
    /// Creates a new [`BrowserHistory`]
    pub fn new() -> Self {
        Self::default()
    }

    fn notify_callbacks(&self) {
        crate::utils::notify_callbacks(self.callbacks.clone());
    }

    fn create_state(user_state: Option<JsValue>) -> JsValue {
        compose_state(get_id(), user_state)
    }

    pub(crate) fn combine_url(route: &str, query: &str) -> String {
        let href = window()
            .location()
            .href()
            .expect_throw("Failed to read location href");

        let url = Url::new_with_base(route, &href).expect_throw("current url is not valid.");

        url.set_search(query);

        url.href()
    }
}
