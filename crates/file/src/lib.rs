//!
//! Working with files and blobs on the Web.
//!
//! These APIs come in two flavors:
//!
//! 1. a callback style (that more directly mimics the JavaScript APIs), and
//! 2. a `Future` API.

mod blob;
mod file_list;
mod file_reader;
mod object_url;

pub use blob::*;
pub use file_list::*;
pub use file_reader::*;
pub use object_url::*;

/// Bound carrying the unwind-safety requirement on [`callbacks`] functions.
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

#[doc(hidden)]
#[cfg(not(all(target_arch = "wasm32", panic = "unwind")))]
pub trait CallbackUnwindSafe {}
#[cfg(not(all(target_arch = "wasm32", panic = "unwind")))]
impl<T> CallbackUnwindSafe for T {}

mod sealed {
    pub trait Sealed {}
}
use sealed::Sealed;
