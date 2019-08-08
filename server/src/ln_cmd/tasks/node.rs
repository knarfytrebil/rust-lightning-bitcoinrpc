use crate::ln_cmd::tasks::{udp_srv, ln_mgr};
use crate::ln_cmd::tasks::{Arg, Probe, TaskFn};
use crate::ln_manager::executor::Larva;

// TODO: Make argument more readable
// arg.0 = ln_conf
// arg.1 = node_conf
fn node(arg: Vec<Arg>, exec: Probe) -> Result<(), String> {

    // run udp_srv and ln_mgr 
    let spawn_ln_mgr = ln_mgr::gen(vec![arg[0].clone()], exec.clone());
    let executor = exec.clone();
    let _ = exec.spawn_task(async move { 
        let ln_mgr = spawn_ln_mgr.await?;
        let spawn_udp_srv = udp_srv::gen(vec![arg[1].clone()], executor.clone(), ln_mgr);
        let _ = spawn_udp_srv.await;
        Ok(())
    });

    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}

pub async fn run_forever() -> Result<(), failure::Error> {
    loop { }
}
