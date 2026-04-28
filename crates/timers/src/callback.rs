//! Callback-style timer APIs.

use js_sys::Function;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};

/// Bound carrying the unwind-safety requirement on [`Timeout::new`] /
/// [`Interval::new`] callbacks.
///
/// Under `panic = "unwind"` on wasm the callback is invoked across a
/// `catch_unwind` boundary inside `wasm_bindgen`, so this resolves to
/// [`std::panic::UnwindSafe`]. Under any other panic strategy it is a no-op
/// blanket. Wrap non-`UnwindSafe` captures in [`std::panic::AssertUnwindSafe`]
/// at the call site.
#[cfg(all(target_arch = "wasm32", panic = "unwind"))]
pub trait CallbackUnwindSafe: std::panic::UnwindSafe {}
#[cfg(all(target_arch = "wasm32", panic = "unwind"))]
impl<T: std::panic::UnwindSafe> CallbackUnwindSafe for T {}

/// Bound carrying the unwind-safety requirement on [`Timeout::new`] /
/// [`Interval::new`] callbacks.
///
/// Under `panic = "unwind"` on wasm the callback is invoked across a
/// `catch_unwind` boundary inside `wasm_bindgen`, so this resolves to
/// [`std::panic::UnwindSafe`]. Under any other panic strategy it is a no-op
/// blanket. Wrap non-`UnwindSafe` captures in [`std::panic::AssertUnwindSafe`]
/// at the call site.
#[cfg(not(all(target_arch = "wasm32", panic = "unwind")))]
pub trait CallbackUnwindSafe {}
#[cfg(not(all(target_arch = "wasm32", panic = "unwind")))]
impl<T> CallbackUnwindSafe for T {}

#[wasm_bindgen]
unsafe extern "C" {
    #[wasm_bindgen(js_name = "setTimeout", catch)]
    fn set_timeout(handler: &Function, timeout: i32) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = "setInterval", catch)]
    fn set_interval(handler: &Function, timeout: i32) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = "clearTimeout")]
    fn clear_timeout(handle: JsValue) -> JsValue;

    #[wasm_bindgen(js_name = "clearInterval")]
    fn clear_interval(handle: JsValue) -> JsValue;
}

/// A scheduled timeout.
///
/// See `Timeout::new` for scheduling new timeouts.
///
/// Once scheduled, you can [`drop`] the [`Timeout`] to clear it or [`forget`](Timeout::forget) to leak it. Once forgotten, the interval will keep running forever.
/// This pattern is known as Resource Acquisition Is Initialization (RAII).
#[derive(Debug)]
#[must_use = "timeouts cancel on drop; either call `forget` or `drop` explicitly"]
pub struct Timeout {
    id: Option<JsValue>,
    closure: Option<Closure<dyn FnMut()>>,
}

impl Drop for Timeout {
    /// Disposes of the timeout, dually cancelling this timeout by calling
    /// `clearTimeout` directly.
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            clear_timeout(id);
        }
    }
}

impl Timeout {
    /// Schedule a timeout to invoke `callback` in `millis` milliseconds from
    /// now.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gloo_timers::callback::Timeout;
    ///
    /// let timeout = Timeout::new(1_000, move || {
    ///     // Do something...
    /// });
    /// ```
    pub fn new<F>(millis: u32, callback: F) -> Timeout
    where
        F: 'static + FnOnce() + CallbackUnwindSafe,
    {
        // Under `panic = "unwind"` we use the `_assert_unwind_safe` variant
        // because `WasmClosureFnOnce` selection erases `F` into a trait-object
        // dispatch that no longer carries the static `UnwindSafe` bound — the
        // same dyn-erasure problem wasm-bindgen handles internally. The
        // `CallbackUnwindSafe` bound on the public API has already enforced
        // the requirement at the call site. On any other panic strategy
        // `Closure::once` is unchanged and works on any 0.2.x wasm-bindgen.
        #[cfg(all(target_arch = "wasm32", panic = "unwind"))]
        let closure = Closure::once_assert_unwind_safe(callback);
        #[cfg(not(all(target_arch = "wasm32", panic = "unwind")))]
        let closure = Closure::once(callback);

        let id = set_timeout(
            closure.as_ref().unchecked_ref::<js_sys::Function>(),
            millis as i32,
        )
        .unwrap_throw();

        Timeout {
            id: Some(id),
            closure: Some(closure),
        }
    }

    /// Forgets this resource without clearing the timeout.
    ///
    /// Returns the identifier returned by the original `setTimeout` call, and
    /// therefore you can still cancel the timeout by calling `clearTimeout`
    /// directly (perhaps via `web_sys::clear_timeout_with_handle`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gloo_timers::callback::Timeout;
    ///
    /// // We definitely want to do stuff, and aren't going to ever cancel this
    /// // timeout.
    /// Timeout::new(1_000, || {
    ///     // Do stuff...
    /// }).forget();
    /// ```
    pub fn forget(mut self) -> JsValue {
        let id = self.id.take().unwrap_throw();
        self.closure.take().unwrap_throw().forget();
        id
    }

    /// Cancel this timeout so that the callback is not invoked after the time
    /// is up.
    ///
    /// The scheduled callback is returned.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gloo_timers::callback::Timeout;
    ///
    /// let timeout = Timeout::new(1_000, || {
    ///     // Do stuff...
    /// });
    ///
    /// // If actually we didn't want to set a timer, then cancel it.
    /// if nevermind() {
    ///     timeout.cancel();
    /// }
    /// # fn nevermind() -> bool { true }
    /// ```
    pub fn cancel(mut self) -> Closure<dyn FnMut()> {
        self.closure.take().unwrap_throw()
    }
}

/// A scheduled interval.
///
/// See `Interval::new` for scheduling new intervals.
///
/// Once scheduled, you can [`drop`] the [`Interval`] to clear it or [`forget`](Interval::forget) to leak it. Once forgotten, the interval will keep running forever.
/// This pattern is known as Resource Acquisition Is Initialization (RAII).
#[derive(Debug)]
#[must_use = "intervals cancel on drop; either call `forget` or `drop` explicitly"]
pub struct Interval {
    id: Option<JsValue>,
    closure: Option<Closure<dyn FnMut()>>,
}

impl Drop for Interval {
    /// Disposes of the interval, dually cancelling this interval by calling
    /// `clearInterval` directly.
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            clear_interval(id);
        }
    }
}

impl Interval {
    /// Schedule an interval to invoke `callback` every `millis` milliseconds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gloo_timers::callback::Interval;
    ///
    /// let interval = Interval::new(1_000, move || {
    ///     // Do something...
    /// });
    /// ```
    pub fn new<F>(millis: u32, callback: F) -> Interval
    where
        F: 'static + FnMut() + CallbackUnwindSafe,
    {
        // Same rationale as `Timeout::new`: the `Box<F> as Box<dyn FnMut()>`
        // coercion erases the `UnwindSafe` bound, so under `panic = "unwind"`
        // we use `_assert_unwind_safe` to acknowledge the erasure (the public
        // `CallbackUnwindSafe` bound has already enforced unwind safety at
        // the call site). Otherwise the original `Closure::wrap` path is
        // preserved for older wasm-bindgen compatibility.
        #[cfg(all(target_arch = "wasm32", panic = "unwind"))]
        let closure = Closure::wrap_assert_unwind_safe(Box::new(callback) as Box<dyn FnMut()>);
        #[cfg(not(all(target_arch = "wasm32", panic = "unwind")))]
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut()>);

        let id = set_interval(
            closure.as_ref().unchecked_ref::<js_sys::Function>(),
            millis as i32,
        )
        .unwrap_throw();

        Interval {
            id: Some(id),
            closure: Some(closure),
        }
    }

    /// Forget this resource without clearing the interval.
    ///
    /// Returns the identifier returned by the original `setInterval` call, and
    /// therefore you can still cancel the interval by calling `clearInterval`
    /// directly (perhaps via `web_sys::clear_interval_with_handle`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gloo_timers::callback::Interval;
    ///
    /// // We want to do stuff every second, indefinitely.
    /// Interval::new(1_000, || {
    ///     // Do stuff...
    /// }).forget();
    /// ```
    pub fn forget(mut self) -> JsValue {
        let id = self.id.take().unwrap_throw();
        self.closure.take().unwrap_throw().forget();
        id
    }

    /// Cancel this interval so that the callback is no longer periodically
    /// invoked.
    ///
    /// The scheduled callback is returned.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gloo_timers::callback::Interval;
    ///
    /// let interval = Interval::new(1_000, || {
    ///     // Do stuff...
    /// });
    ///
    /// // If we don't want this interval to run anymore, then cancel it.
    /// if nevermind() {
    ///     interval.cancel();
    /// }
    /// # fn nevermind() -> bool { true }
    /// ```
    pub fn cancel(mut self) -> Closure<dyn FnMut()> {
        self.closure.take().unwrap_throw()
    }
}
