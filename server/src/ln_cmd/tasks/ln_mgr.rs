use crate::ln_cmd::tasks::Arg;
use crate::ln_cmd::tasks::Probe;
use futures::future::Future;
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_manager::LnManager;

pub fn gen(
    arg: Vec<Arg>,
    exec: Probe,
) -> impl Future<Output = Result<LnManager<Probe>, ()>> + Send + 'static {

    let ln_conf: Option<&MgrSettings> = match &arg[0] {
        Arg::MgrConf(conf) => Some(conf),
        _ => None,
    };

    LnManager::new(
        ln_conf.unwrap().clone(), 
        exec.clone()
    )
}
