use ln_cmd::tasks::{Arg, TaskFn};
use ln_cmd::tasks::{ProbT, Probe};
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_manager::LnManager;

use std::thread;

pub fn task(arg: Vec<Arg>) -> Result<(), String> {
    let ln_conf: Option<&MgrSettings> = match &arg[0] {
        Arg::MgrConf(conf) => Some(conf),
        _ => None,
    };

    let inner_runner = Probe::new(ProbT::NonBlocking);

    let ln_manager = LnManager::new(ln_conf.unwrap().clone(), inner_runner);

    // exit here
    // FIXME: Unreachable
    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(task)
}
