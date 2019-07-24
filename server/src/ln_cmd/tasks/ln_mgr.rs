use ln_cmd::tasks::{Arg, TaskFn};
use ln_cmd::tasks::{ProbeT, Probe};
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_manager::LnManager;
use futures::future::Future;
use futures::channel::mpsc;

pub fn task(arg: Vec<Arg>, exec: Probe) -> Result<(), String> {
    let ln_conf: Option<&MgrSettings> = match &arg[0] {
        Arg::MgrConf(conf) => Some(conf),
        _ => None,
    };


    // let ln_manager = LnManager::new(ln_conf.unwrap().clone(), exec.clone());

    // exit here
    // FIXME: Unreachable
    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(task)
}
