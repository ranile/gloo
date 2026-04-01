use gloo_worker::Registrable;
use nested_worker_yew::CoordinatorWorker;

fn main() {
    console_error_panic_hook::set_once();
    CoordinatorWorker::registrar().register();
}
