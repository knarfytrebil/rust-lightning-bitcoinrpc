use ln_cmd::tasks::{udp_srv, ln_mgr};
use ln_cmd::tasks::{Action, Arg, ProbeT, Probe, TaskFn};
use futures::future::Future;
use futures::sync::mpsc;
use ln_manager::executor::Larva;

// TODO: Make argument more readable
// arg.0 = ln_conf
// arg.1 = node_conf
fn node(arg: Vec<Arg>) -> Result<(), String> {

    // run udp server
    let (udp_tx, udp_rx) = mpsc::unbounded::<Box<dyn Future<Item = (), Error = ()> + Send>>();
    let udp_runner = Probe::new(ProbeT::NonBlocking, udp_tx);
    let udp_srv: Action = Action::new(udp_srv::gen, vec![arg[1].clone()]);
    let _ = udp_runner.spawn_task(udp_srv);

    // run ln manager
    let (ln_tx, ln_rx) = mpsc::unbounded::<Box<dyn Future<Item = (), Error = ()> + Send>>();
    let ln_mgr_runner = Probe::new(ProbeT::NonBlocking, ln_tx);
    let ln_mgr: Action = Action::new(ln_mgr::gen, vec![arg[0].clone()]);
    let _ = ln_mgr_runner.spawn_task(ln_mgr);

    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}
