use ln_cmd::tasks::{udp_srv, ln_mgr};
use ln_cmd::tasks::{Action, Arg, ProbT, Probe, TaskFn};
use ln_manager::executor::Larva;

// TODO: Make argument more readable
// arg.0 = ln_conf
// arg.1 = node_conf
fn node(arg: Vec<Arg>) -> Result<(), String> {
    let udp_runner = Probe::new(ProbT::NonBlocking);
    let udp_srv: Action = Action::new(udp_srv::gen, vec![arg[1].clone()]);
    let _ = udp_runner.spawn_task(udp_srv);

    let ln_mgr_runner = Probe::new(ProbT::NonBlocking);
    let ln_mgr: Action = Action::new(ln_mgr::gen, vec![arg[0].clone()]);
    let _ = ln_mgr_runner.spawn_task(ln_mgr);

    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}
