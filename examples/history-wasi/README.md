# History example on WASI

This is a simple example showcasing the Gloo History on WASI.

You can run this example with:

```bash
cargo build --manifest-path examples/history-wasi/Cargo.toml --target wasm32-wasip1
wasmtime examples/history-wasi/target/wasm32-wasip1/debug/example-history-wasi.wasm
```
