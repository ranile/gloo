pub mod app;

use std::collections::HashMap;

use gloo_worker::{HandlerId, Spawnable, Worker, WorkerBridge, WorkerScope};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FibRequest(pub u32);

#[derive(Serialize, Deserialize, Clone)]
pub struct FibResult {
    pub n: u32,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub worker: String,
    pub event: String,
}

#[derive(Serialize, Deserialize)]
pub enum ComputeOutput {
    Log(String),
    Result(FibResult),
}

pub struct ComputeWorker;

impl Worker for ComputeWorker {
    type Message = ();
    type Input = FibRequest;
    type Output = ComputeOutput;

    fn create(_scope: &WorkerScope<Self>) -> Self {
        Self
    }

    fn update(&mut self, _scope: &WorkerScope<Self>, _msg: Self::Message) {}

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        let n = msg.0;
        scope.respond(id, ComputeOutput::Log(format!("Received fib({n})")));

        let start = js_sys::Date::now();
        let value = fibonacci(n);
        let elapsed = js_sys::Date::now() - start;

        scope.respond(id, ComputeOutput::Log(format!("Computed ({elapsed:.0}ms)")));
        scope.respond(id, ComputeOutput::Result(FibResult { n, value }));
    }
}

fn fibonacci(n: u32) -> u64 {
    if n <= 1 {
        return n as u64;
    }
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 2..=n {
        let c = a.wrapping_add(b);
        a = b;
        b = c;
    }
    b
}

#[derive(Serialize, Deserialize)]
pub enum CoordinatorOutput {
    Log(LogEntry),
    Result(FibResult),
    WorkerSpawned(u32),
    WorkerFinished(u32),
}

pub enum CoordinatorMsg {
    FromCompute(u32, ComputeOutput),
}

pub struct CoordinatorWorker {
    next_id: u32,
    bridge_id: Option<HandlerId>,
    active: HashMap<u32, WorkerBridge<ComputeWorker>>,
}

impl CoordinatorWorker {
    fn emit(&self, scope: &WorkerScope<Self>, output: CoordinatorOutput) {
        if let Some(id) = self.bridge_id {
            scope.respond(id, output);
        }
    }
}

impl Worker for CoordinatorWorker {
    type Message = CoordinatorMsg;
    type Input = FibRequest;
    type Output = CoordinatorOutput;

    fn create(_scope: &WorkerScope<Self>) -> Self {
        Self {
            next_id: 0,
            bridge_id: None,
            active: HashMap::new(),
        }
    }

    fn update(&mut self, scope: &WorkerScope<Self>, msg: Self::Message) {
        match msg {
            CoordinatorMsg::FromCompute(cid, ComputeOutput::Log(text)) => {
                self.emit(
                    scope,
                    CoordinatorOutput::Log(LogEntry {
                        worker: format!("Worker #{cid}"),
                        event: text,
                    }),
                );
            }
            CoordinatorMsg::FromCompute(cid, ComputeOutput::Result(result)) => {
                self.active.remove(&cid);
                self.emit(
                    scope,
                    CoordinatorOutput::Log(LogEntry {
                        worker: "Coordinator".into(),
                        event: format!("Result from worker #{cid}"),
                    }),
                );
                self.emit(scope, CoordinatorOutput::WorkerFinished(cid));
                self.emit(scope, CoordinatorOutput::Result(result));
            }
        }
    }

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        self.bridge_id = Some(id);
        let n = msg.0;
        let cid = self.next_id;
        self.next_id += 1;

        let cb =
            scope.callback(move |output: ComputeOutput| CoordinatorMsg::FromCompute(cid, output));
        let mut spawner = ComputeWorker::spawner();
        spawner.callback(move |output| cb(output)).with_loader(true);
        let bridge = spawner.spawn("/nested_worker_inner_loader.js");
        bridge.send(FibRequest(n));
        self.active.insert(cid, bridge);

        self.emit(scope, CoordinatorOutput::WorkerSpawned(cid));
        self.emit(
            scope,
            CoordinatorOutput::Log(LogEntry {
                worker: "Coordinator".into(),
                event: format!("Spawned worker #{cid} for fib({n})"),
            }),
        );
    }
}
