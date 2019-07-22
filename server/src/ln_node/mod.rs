pub mod settings;

use ln_cmd::tasks::node;
use ln_cmd::tasks::{Action, Arg, ProbeT, Probe};
use ln_manager::executor::Larva;
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_node::settings::Settings as NodeSettings;
use futures::future::Future;
use futures::channel::mpsc;


pub fn run(ln_conf: MgrSettings, node_conf: NodeSettings) {
    // println!("{:#?}", ln_conf);
    // println!("{:#?}", node_conf);

    let (node_tx, node_rx) = mpsc::unbounded::<Box<dyn Future<Item = (), Error = ()> + Send>>();
    let run_forever = Probe::new(ProbeT::Blocking, node_tx);
    let init_node: Action = Action::new(
        node::gen,
        vec![Arg::MgrConf(ln_conf), Arg::NodeConf(node_conf)],
    );
    let _ = run_forever.spawn_task(init_node);
}
