/// Calls `console.clear()`
#[macro_export]
macro_rules! clear {
    () => {
        $crate::externs::clear()
    };
}

/// Calls `console.assert()`
#[macro_export]
macro_rules! assert {
    ($assertion:expr, $($arg:expr),+) => {
       $crate::externs::assert($assertion, ::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    }
}

/// Calls `console.debug()`
#[macro_export]
macro_rules! debug {
    ($($arg:expr),+) => {
       $crate::externs::debug(::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    }
}

/// Calls `console.dir()`
/// ## Example
/// ```no_run
/// # use gloo_console::dir;
/// # use js_sys::Array;
/// dir!(Array::of2(&1.into(), &2.into()));
/// ```
#[macro_export]
macro_rules! dir {
    ($arg:expr) => {
        $crate::externs::dir(&$crate::__macro::JsValue::from($arg))
    };
}

/// Calls `console.dirxml()`
/// ## Example
/// ```no_run
/// # use gloo_console::dirxml;
/// # use js_sys::Array;
/// dirxml!(Array::of2(&1.into(), &2.into()));
/// ```
#[macro_export]
macro_rules! dirxml {
    ($arg:expr) => {
        $crate::externs::dirxml(&$crate::__macro::JsValue::from($arg))
    };
}

/// Calls `console.error()`
#[macro_export]
macro_rules! error {
    ($($arg:expr),+) => {
       $crate::externs::error(::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    }
}

/// Calls `console.group()`
///
/// In order to call `console.groupCollapsed`, prefix the arguments with `collapsed`.
#[macro_export]
macro_rules! group {
    ($($arg:expr),+) => {
       $crate::externs::group(::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    };
    (collapsed $($arg:expr),+) => {
       $crate::externs::group_collapsed(::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    };
}

/// Calls `console.groupEnd()`
#[macro_export]
macro_rules! group_end {
    () => {
        $crate::externs::group_end()
    };
}

/// Calls `console.info()`
#[macro_export]
macro_rules! info {
    ($($arg:expr),+) => {
       $crate::externs::info(::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    }
}

/// Calls `console.table()`
///
/// Since in most cases, this takes in an object, instead of, say a string literal/variable,
/// we use [`serde`](https://serde.rs) to serialize the passed data object into
/// [`JsValue`][wasm_bindgen::JsValue].
///
/// An `IntoIterator<Item = &str>` can be passed to specify the columns.
#[macro_export]
macro_rules! table {
    ($data:expr) => {
        $crate::externs::table_with_data($crate::__macro::JsValue::from($data))
    };
    ($data:expr, $columns:expr) => {
        $crate::__macro::table_with_data_and_columns($data, $columns)
    };
}

/// Calls `console.log()`
#[macro_export]
macro_rules! log {
    ($($arg:expr),+) => {
       $crate::externs::log(::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    }
}

/// Calls `console.trace()`
#[macro_export]
macro_rules! trace {
    ($($arg:expr),+) => {
       $crate::externs::trace(::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    }
}

/// Calls `console.warn()`
#[macro_export]
macro_rules! warn {
    ($($arg:expr),+) => {
       $crate::externs::warn(::std::boxed::Box::from([$($crate::__macro::JsValue::from($arg),)+]))
    }
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    //! These exist to ensure macros compile in expression position
    //! (no trailing semicolon in the expansion, see rust-lang/rust#79813)

    fn expression_position_works() {
        let _: () = crate::clear!();
        let _: () = crate::assert!(true, "msg");
        let _: () = crate::debug!("debug");
        let _: () = crate::dir!("dir");
        let _: () = crate::dirxml!("dirxml");
        let _: () = crate::error!("error");
        let _: () = crate::group!("group");
        let _: () = crate::group!(collapsed "group");
        let _: () = crate::group_end!();
        let _: () = crate::info!("info");
        let _: () = crate::table!(crate::__macro::JsValue::from("data"));
        let _: () = crate::log!("log");
        let _: () = crate::trace!("trace");
        let _: () = crate::warn!("warn");
        let _: () = crate::console!();
    }
}
