use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

/// Message Encoding and Decoding Format
pub trait Codec {
    /// Encode an input to JsValue
    fn encode<I>(input: I) -> JsValue
    where
        I: Serialize;

    /// Decode a message to a type
    fn decode<O>(input: JsValue) -> O
    where
        O: for<'de> Deserialize<'de>;
}

/// Default message encoding with [postcard].
#[derive(Debug)]
pub struct Postcard;

impl Codec for Postcard {
    fn encode<I>(input: I) -> JsValue
    where
        I: Serialize,
    {
        let buf = postcard::to_stdvec(&input).expect("can't serialize a worker message");
        Uint8Array::from(buf.as_slice()).into()
    }

    fn decode<O>(input: JsValue) -> O
    where
        O: for<'de> Deserialize<'de>,
    {
        let data = Uint8Array::from(input).to_vec();
        postcard::from_bytes(&data).expect("can't deserialize a worker message")
    }
}
