use wasm_bindgen_test::{wasm_bindgen_test as test, wasm_bindgen_test_configure};

use gloo_history::{HashHistory, History};
use gloo_utils::window;

wasm_bindgen_test_configure!(run_in_browser);

mod utils;
use utils::delayed_assert_eq;

// All assertions live in a single test because HashHistory is a thread-local
// singleton backed by a shared browser URL, so separate tests would leak
// state into each other depending on execution order.
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

    // Malformed hash: simulate user editing the URL bar to a hash without '/'
    window().location().set_hash("no-leading-slash").unwrap();

    let location = history.location();
    assert_eq!(location.path(), "/no-leading-slash");

    delayed_assert_eq(
        || window().location().hash().unwrap(),
        || "#/no-leading-slash",
    )
    .await;

    // Empty hash: simulate user clearing the hash entirely
    window().location().set_hash("").unwrap();

    let location = history.location();
    assert_eq!(location.path(), "/");

    delayed_assert_eq(|| window().location().hash().unwrap(), || "#/").await;
}
