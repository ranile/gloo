use gloo_storage::AsyncStorage;
use gloo_storage::CacheStorage;
use serde::{Deserialize, Serialize};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn get_and_set() {
    let key = "https://rustacean.net/assets/cuddlyferris.png";
    let value = "Ferris is cute 🦀";

    CacheStorage::set(key, value).await.unwrap();
    let obtained: String = CacheStorage::get(key).await.unwrap();

    assert_eq!(obtained, value);
}

#[derive(Serialize, Deserialize)]
struct FerrisFacts {
    cuteness: String,
    power: String,
}

#[wasm_bindgen_test]
async fn get_all() {
    CacheStorage::set(
        "https://rustacean.net/assets/cuddlyferris.png",
        "Ferris is cute 🦀",
    )
    .await
    .unwrap();
    CacheStorage::set("power", "Ferris is King 👑")
        .await
        .unwrap();

    let facts: serde_json::Value = CacheStorage::get_all().await.unwrap();

    assert_eq!(
        facts["https://rustacean.net/assets/cuddlyferris.png"],
        "Ferris is cute 🦀"
    );
    assert_eq!(facts["power"], "Ferris is King 👑");
}

#[wasm_bindgen_test]
async fn set_and_length() {
    CacheStorage::clear().await.unwrap();

    let len = CacheStorage::length().await.unwrap();
    assert_eq!(len, 0);

    CacheStorage::set(
        "https://rustacean.net/assets/cuddlyferris.png",
        "Trust the compiler, no cap 🧠",
    )
    .await
    .unwrap();
    let len = CacheStorage::length().await.unwrap();
    assert_eq!(len, 1);

    CacheStorage::clear().await.unwrap();
    let len = CacheStorage::length().await.unwrap();
    assert_eq!(len, 0);
}

#[wasm_bindgen_test]
async fn delete_key() {
    let key = "https://rustacean.net/assets/cuddlyferris.png";
    CacheStorage::set(key, "Goodbye, Ferris, see you tomorrow 😢")
        .await
        .unwrap();
    assert!(CacheStorage::get::<String>(key).await.is_ok());

    CacheStorage::delete(key).await.unwrap();
    let result = CacheStorage::get::<String>(key).await;

    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn clear_storage() {
    CacheStorage::set(
        "https://rustacean.net/assets/cuddlyferris.png",
        "Ferris remembers everything 🧠",
    )
    .await
    .unwrap();
    CacheStorage::set(
        "https://rustacean.net/assets/cuddlyferris.png",
        "Except when cleared, no cap 😅",
    )
    .await
    .unwrap();

    let len_before = CacheStorage::length().await.unwrap();
    assert_eq!(len_before, 2);

    CacheStorage::clear().await.unwrap();

    let len_after = CacheStorage::length().await.unwrap();
    assert_eq!(len_after, 0);
}
