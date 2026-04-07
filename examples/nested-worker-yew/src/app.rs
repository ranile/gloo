use std::rc::Rc;

use gloo_worker::Spawnable;
use web_sys::MouseEvent;
use yew::prelude::*;

use crate::{CoordinatorOutput, CoordinatorWorker, FibRequest, FibResult, LogEntry};

#[derive(Clone)]
struct AppState {
    results: Vec<FibResult>,
    active_workers: Vec<u32>,
    last_event: Option<LogEntry>,
}

enum AppAction {
    Log(LogEntry),
    Result(FibResult),
    WorkerSpawned(u32),
    WorkerFinished(u32),
}

impl Reducible for AppState {
    type Action = AppAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut new = (*self).clone();
        match action {
            AppAction::Log(entry) => new.last_event = Some(entry),
            AppAction::Result(result) => new.results.push(result),
            AppAction::WorkerSpawned(id) => new.active_workers.push(id),
            AppAction::WorkerFinished(id) => new.active_workers.retain(|&w| w != id),
        }
        Rc::new(new)
    }
}

#[component]
pub fn App() -> Html {
    let state = use_reducer(|| AppState {
        results: Vec::new(),
        active_workers: Vec::new(),
        last_event: None,
    });

    let bridge = use_memo((), {
        let state = state.clone();
        move |_| {
            CoordinatorWorker::spawner()
                .callback(move |output: CoordinatorOutput| match output {
                    CoordinatorOutput::Log(e) => state.dispatch(AppAction::Log(e)),
                    CoordinatorOutput::Result(r) => state.dispatch(AppAction::Result(r)),
                    CoordinatorOutput::WorkerSpawned(id) => {
                        state.dispatch(AppAction::WorkerSpawned(id))
                    }
                    CoordinatorOutput::WorkerFinished(id) => {
                        state.dispatch(AppAction::WorkerFinished(id))
                    }
                })
                .with_loader(true)
                .spawn("/nested_worker_outer_loader.js")
        }
    });

    let send = |lo: u32, hi: u32| {
        let bridge = bridge.clone();
        Callback::from(move |_: MouseEvent| {
            bridge.send(FibRequest(rand::random_range(lo..hi)));
        })
    };

    let status = if state.active_workers.is_empty() {
        "Idle".to_string()
    } else {
        format!("{} worker(s) active", state.active_workers.len())
    };

    html! {
        <main>
            <h1>{"Nested Web Worker"}</h1>
            <p class="subtitle">{"Fibonacci computed through a chain of nested web workers"}</p>

            <div class="controls">
                <button onclick={send(5_000_000, 20_000_000)}>{"~10M"}</button>
                <button onclick={send(50_000_000, 200_000_000)}>{"~100M"}</button>
                <button onclick={send(300_000_000, 700_000_000)}>{"~500M"}</button>
            </div>

            <div class="workers">
                <div class={if state.active_workers.is_empty() { "worker-node worker-node--coordinator" } else { "worker-node worker-node--coordinator active" }}>
                    <div class="worker-node__icon">{"C"}</div>
                    <div class="worker-node__label">{"Coordinator"}</div>
                    <div class="worker-node__status">{status}</div>
                </div>
                <div class="workers__connector"></div>
                <div class="workers__pool">
                    for &id in state.active_workers.iter() {
                        <div class="worker-node worker-node--compute" key={id.to_string()}>
                            <div class="worker-node__icon">{format!("#{id}")}</div>
                            <div class="worker-node__label">{"Compute"}</div>
                        </div>
                    }
                </div>
            </div>

            <div class="panel">
                <h2 class="panel__title">{"Results"}</h2>
                <div class="results">
                    if state.results.is_empty() {
                        <p class="results__empty">{"Waiting for computation..."}</p>
                    }
                    { for state.results.iter().rev().take(6).map(render_result) }
                </div>
            </div>
        </main>
    }
}

fn render_result(r: &FibResult) -> Html {
    html! {
        <div class="result-entry">
            <span class="result-entry__fn">{format!("fib({})", r.n)}</span>
            <span class="result-entry__eq">{" = "}</span>
            <span class="result-entry__val">{r.value.to_string()}</span>
        </div>
    }
}
