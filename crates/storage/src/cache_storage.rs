use crate::AsyncStorage;
use js_sys::Array;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::future::Future;
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;
use web_sys::{Cache, CacheStorage as WebCacheStorage, Request, Response};

use crate::errors::StorageError;
use crate::Result;

/// Provides API to deal with `CacheStorage`
#[derive(Debug)]
pub struct CacheStorage;

impl CacheStorage {
    fn raw() -> WebCacheStorage {
        window().expect_throw("no window").caches().unwrap_throw()
    }

    fn make_request(url: &str) -> Result<Request> {
        Request::new_with_str(&url)
            .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))
    }

    async fn open_cache() -> Result<Cache> {
        let promise = Self::raw().open("gloo-cache");
        let cache = JsFuture::from(promise)
            .await
            .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?;
        Ok(Cache::from(cache))
    }

    async fn all_keys() -> Result<Vec<String>> {
        let promise = Self::raw().keys();
        let js_value = JsFuture::from(promise)
            .await
            .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?;
        let array: Array = js_value.dyn_into().unwrap_throw();
        Ok(array.iter().filter_map(|v| v.as_string()).collect())
    }
}

impl AsyncStorage for CacheStorage {
    fn get<T>(key: &str) -> impl Future<Output = Result<T>>
    where
        T: for<'de> Deserialize<'de> + 'static,
    {
        let key = key.to_string();
        async move {
            let cache = Self::open_cache().await?;
            let req = Self::make_request(&key)?;

            let match_promise = cache.match_with_request(&req);
            let res_val = JsFuture::from(match_promise)
                .await
                .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?;

            if res_val.is_undefined() {
                return Err(StorageError::KeyNotFound(key));
            }

            let response: Response = res_val.dyn_into().unwrap_throw();
            let text_promise = response
                .text()
                .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?;
            let text = JsFuture::from(text_promise)
                .await
                .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?
                .as_string()
                .ok_or_else(|| {
                    StorageError::SerdeError(serde_json::Error::custom("Expected response text"))
                })?;

            Ok(serde_json::from_str(&text)?)
        }
    }

    fn get_all<T>() -> impl Future<Output = Result<T>>
    where
        T: for<'de> Deserialize<'de> + 'static,
    {
        async move {
            let keys = Self::all_keys().await?;
            let mut map = Map::with_capacity(keys.len());
            for key in keys {
                let val: Value = Self::get(&key).await?;
                map.insert(key, val);
            }
            Ok(serde_json::from_value(Value::Object(map))?)
        }
    }

    fn set<T>(key: &str, value: T) -> impl Future<Output = Result<()>>
    where
        T: Serialize + 'static,
    {
        let key = key.to_string();
        async move {
            let cache = Self::open_cache().await?;
            let req = Self::make_request(&key)?;
            let json = serde_json::to_string(&value)?;
            let res = Response::new_with_opt_str(Some(&json))
                .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?;

            let put_promise = cache.put_with_request(&req, &res);
            JsFuture::from(put_promise)
                .await
                .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?;

            Ok(())
        }
    }

    fn delete(key: &str) -> impl Future<Output = Result<()>> {
        let key = key.to_string();
        async move {
            let cache = Self::open_cache().await?;
            let req = Self::make_request(&key)?;
            let delete_promise = cache.delete_with_request(&req);
            JsFuture::from(delete_promise)
                .await
                .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?;
            Ok(())
        }
    }

    fn clear() -> impl Future<Output = Result<()>> {
        async move {
            let delete_promise = Self::raw().delete("gloo-cache");
            JsFuture::from(delete_promise)
                .await
                .map_err(|e| StorageError::JsError(js_sys::Error::from(e).into()))?;
            Ok(())
        }
    }

    fn length() -> impl Future<Output = Result<u32>> {
        async move { Ok(Self::all_keys().await?.len() as u32) }
    }
}
