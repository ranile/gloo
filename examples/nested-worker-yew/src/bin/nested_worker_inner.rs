use gloo_worker::Registrable;
use nested_worker_yew::ComputeWorker;

fn main() {
    console_error_panic_hook::set_once();
    ComputeWorker::registrar().register();
}
