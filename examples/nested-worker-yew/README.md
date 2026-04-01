# Nested Web Worker Example

Demonstrates spawning a web worker from inside another web worker using `gloo-worker`.

A **Coordinator** worker receives requests from the main thread and delegates computation
to a nested **Compute** worker. The UI visualizes both workers' lifecycles in real time.

## Running

install [trunk](https://trunk-rs.github.io/trunk/)

```bash
trunk serve
```
