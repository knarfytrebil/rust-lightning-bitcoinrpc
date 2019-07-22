use ln_cmd::tasks::{Arg, TaskFn};
use ln_cmd::tasks::{ProbeT, Probe};
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_manager::LnManager;
use futures::future::Executor;
use futures::future::Future;
use futures::sync::mpsc;


use std::thread;

pub fn task(arg: Vec<Arg>) -> Result<(), String> {
    let ln_conf: Option<&MgrSettings> = match &arg[0] {
        Arg::MgrConf(conf) => Some(conf),
        _ => None,
    };

    let (mgr_tx, mgr_rx) = mpsc::unbounded::<Box<dyn Future<Item = (), Error = ()> + Send>>();
    let inner_runner = Probe::new(ProbeT::NonBlocking, mgr_tx);

    let ln_manager = LnManager::new(ln_conf.unwrap().clone(), inner_runner);

    // exit here
    // FIXME: Unreachable
    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(task)
}
