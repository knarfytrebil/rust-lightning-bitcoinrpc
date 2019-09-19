use crate::ln_cmd::tasks::{ln_mgr, udp_srv};
use crate::ln_cmd::tasks::{Arg, Probe, TaskFn};
use crate::ln_manager::executor::Larva;

fn node(mut args: Vec<Arg>, exec: Probe) -> Result<(), String> {
    let executor = exec.clone();

    let ln_conf: Vec<_> = args
        .splice(..1, [].iter().cloned())
        .collect();
    let node_conf = args;
    let _ = exec.spawn_task(async move {
        let ln_mgr = ln_mgr::gen(ln_conf, executor.clone()).await?;
        let _ = udp_srv::gen(node_conf, executor.clone(), ln_mgr).await;
        Ok(())
    });
    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}

pub async fn run_forever() -> Result<(), failure::Error> {
    loop {}
}
