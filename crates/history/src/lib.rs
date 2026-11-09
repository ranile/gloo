//! A module that provides universal session history and location information.

#![deny(clippy::all)]
#![deny(missing_docs, missing_debug_implementations)]

mod any;
mod browser;
#[cfg(feature = "query")]
mod error;
mod hash;
mod history;
mod listener;
mod location;
mod memory;
#[cfg(feature = "query")]
pub mod query;
mod state;
mod utils;

pub use any::AnyHistory;
pub use browser::BrowserHistory;
pub use hash::HashHistory;
pub use memory::MemoryHistory;

#[cfg(feature = "query")]
pub use error::{HistoryError, HistoryResult};
pub use history::History;
pub use listener::HistoryListener;
pub use location::Location;

/// Bound carrying the unwind-safety requirement on [`History::listen`] callbacks.
///
/// Under `panic = "unwind"` on wasm the callback may be invoked across a
/// `catch_unwind` boundary inside `wasm_bindgen`, so this resolves to
/// [`std::panic::UnwindSafe`]. Under any other panic strategy it is a no-op
/// blanket. Wrap non-`UnwindSafe` captures in [`std::panic::AssertUnwindSafe`]
/// at the call site.
#[cfg(all(target_arch = "wasm32", panic = "unwind"))]
pub trait CallbackUnwindSafe: std::panic::UnwindSafe {}
#[cfg(all(target_arch = "wasm32", panic = "unwind"))]
impl<T: std::panic::UnwindSafe> CallbackUnwindSafe for T {}

#[doc(hidden)]
#[cfg(not(all(target_arch = "wasm32", panic = "unwind")))]
pub trait CallbackUnwindSafe {}
#[cfg(not(all(target_arch = "wasm32", panic = "unwind")))]
impl<T> CallbackUnwindSafe for T {}
