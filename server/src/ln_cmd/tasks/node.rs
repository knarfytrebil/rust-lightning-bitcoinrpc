use crate::ln_cmd::tasks::{udp_srv, ln_mgr};
use crate::ln_cmd::tasks::{Action, Arg, Probe, TaskFn};

// TODO: Make argument more readable
// arg.0 = ln_conf
// arg.1 = node_conf
fn node(arg: Vec<Arg>, exec: Probe) -> Result<(), String> {

    // run udp server
    let udp_srv: Action = Action::new(udp_srv::gen, vec![arg[1].clone()], exec.clone());
    let _ = udp_srv.summon();

    // run ln manager
    let ln_mgr: Action = Action::new(ln_mgr::gen, vec![arg[0].clone()], exec.clone());
    let _ = ln_mgr.summon();

    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}
