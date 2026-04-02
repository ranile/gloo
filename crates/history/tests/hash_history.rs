use wasm_bindgen_test::{wasm_bindgen_test as test, wasm_bindgen_test_configure};

use gloo_history::{HashHistory, History};
use gloo_utils::window;

wasm_bindgen_test_configure!(run_in_browser);

mod utils;
use utils::delayed_assert_eq;

#[test]
async fn history_works() {
    let history = HashHistory::new();

    {
        let history = history.clone();
        delayed_assert_eq(|| history.location().path().to_owned(), || "/").await;
    }
    delayed_assert_eq(|| window().location().pathname().unwrap(), || "/").await;
    delayed_assert_eq(|| window().location().hash().unwrap(), || "#/").await;

    history.push("/path-a");
    {
        let history = history.clone();
        delayed_assert_eq(|| history.location().path().to_owned(), || "/path-a").await;
    }
    delayed_assert_eq(|| window().location().pathname().unwrap(), || "/").await;
    delayed_assert_eq(|| window().location().hash().unwrap(), || "#/path-a").await;

    history.replace("/path-b");
    {
        let history = history.clone();
        delayed_assert_eq(|| history.location().path().to_owned(), || "/path-b").await;
    }
    delayed_assert_eq(|| window().location().pathname().unwrap(), || "/").await;
    delayed_assert_eq(|| window().location().hash().unwrap(), || "#/path-b").await;

    history.back();
    {
        let history = history.clone();
        delayed_assert_eq(|| history.location().path().to_owned(), || "/").await;
    }
    delayed_assert_eq(|| window().location().pathname().unwrap(), || "/").await;
    delayed_assert_eq(|| window().location().hash().unwrap(), || "#/").await;

    history.forward();
    {
        let history = history.clone();
        delayed_assert_eq(|| history.location().path().to_owned(), || "/path-b").await;
    }
    delayed_assert_eq(|| window().location().pathname().unwrap(), || "/").await;
    delayed_assert_eq(|| window().location().hash().unwrap(), || "#/path-b").await;
}

#[test]
async fn location_does_not_panic_on_malformed_hash() {
    let history = HashHistory::new();

    // Simulate the user manually editing the URL bar to a hash without a leading '/'
    window().location().set_hash("no-leading-slash").unwrap();

    // This must NOT panic
    let location = history.location();

    // The path should have been normalized with a leading '/'
    assert_eq!(location.path(), "/no-leading-slash");

    // The URL should have been auto-corrected
    delayed_assert_eq(
        || window().location().hash().unwrap(),
        || "#/no-leading-slash",
    )
    .await;
}

#[test]
async fn location_does_not_panic_on_empty_hash() {
    let history = HashHistory::new();

    // Simulate the user clearing the hash entirely
    window().location().set_hash("").unwrap();

    let location = history.location();

    assert_eq!(location.path(), "/");

    // The URL should have been auto-corrected
    delayed_assert_eq(|| window().location().hash().unwrap(), || "#/").await;
}
