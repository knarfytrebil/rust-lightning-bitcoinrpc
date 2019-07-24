pub mod settings;

use futures::channel::mpsc;
use futures::executor::LocalPool;
use futures::future::Future;
use ln_cmd::executor::Larva;
use ln_cmd::tasks::node;
use ln_cmd::tasks::{Action, Arg, Probe, ProbeT};
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_node::settings::Settings as NodeSettings;
use std::pin::Pin;

pub fn run(ln_conf: MgrSettings, node_conf: NodeSettings) {
    let (pool_tx, mut pool_rx) = mpsc::unbounded::<Pin<Box<dyn Future<Output = ()> + Send>>>();

    let runner = Probe::new(ProbeT::Pool, pool_tx);

    let init_node: Action = Action::new(
        node::gen,
        vec![Arg::MgrConf(ln_conf), Arg::NodeConf(node_conf)],
        runner,
    );

    let _ = init_node.spawn();

    let mut pool = LocalPool::new();

    loop {
        match pool_rx.try_next() {
            Ok(task) => {
                pool.run_until(task.unwrap());
            }
            _ => {}
        }
    }
}
