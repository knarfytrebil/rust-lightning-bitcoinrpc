use crate::ln_cmd::tasks::{Arg, TaskFn};
use crate::ln_cmd::tasks::{Probe};
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_manager::LnManager;

pub fn task(arg: Vec<Arg>, exec: Probe) -> Result<(), String> {
    let ln_conf: Option<&MgrSettings> = match &arg[0] {
        Arg::MgrConf(conf) => Some(conf),
        _ => None,
    };


    let ln_manager = LnManager::new(ln_conf.unwrap().clone(), exec.clone());

    // exit here
    // FIXME: Unreachable
    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(task)
}
