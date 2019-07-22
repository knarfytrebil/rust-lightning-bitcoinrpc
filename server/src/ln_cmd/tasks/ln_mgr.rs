use ln_cmd::tasks::{Arg, TaskFn};
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_manager::LnManager;

use std::thread;

pub fn task(arg: Vec<Arg>) -> Result<(), String> {
    let ln_conf: Option<&MgrSettings> = match &arg[0] {
        Arg::MgrConf(conf) => Some(conf),
        _ => None,
    };

    // exit here
    // FIXME: Unreachable
    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(task)
}
