use ln_cmd::tasks::{udp_srv, ln_mgr};
use ln_cmd::tasks::{Action, Arg, ProbeT, Probe, TaskFn};
use futures::future::Future;
use futures::channel::mpsc;
use ln_cmd::executor::Larva;

// TODO: Make argument more readable
// arg.0 = ln_conf
// arg.1 = node_conf
fn node(arg: Vec<Arg>, exec: Probe) -> Result<(), String> {

    // run udp server
    let udp_srv: Action = Action::new(udp_srv::gen, vec![arg[1].clone()], exec.clone());
    let _ = udp_srv.spawn();

    // run ln manager
    let ln_mgr: Action = Action::new(ln_mgr::gen, vec![arg[0].clone()], exec.clone());
    let _ = ln_mgr.spawn();

    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}
