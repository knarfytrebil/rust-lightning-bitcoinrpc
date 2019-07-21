use ln_cmd::tasks::udp_srv;
use ln_cmd::tasks::{Action, Arg, ProbT, Probe, TaskFn};
use ln_manager::executor::Larva;

// TODO: Make argument more readable
// arg.0 = ln_conf
// arg.1 = node_conf
fn node(arg: Vec<Arg>) -> Result<(), String> {
    let runner = Probe::new(ProbT::NonBlocking);
    let udp_srv: Action = Action::new(udp_srv::gen, vec![arg[1].clone()]);
    let _ = runner.spawn_task(udp_srv);

    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}
