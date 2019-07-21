pub mod settings;

use ln_cmd::tasks::node;
use ln_cmd::tasks::{Action, Arg, ProbT, Probe, TaskFn, TaskGen};
use ln_manager::executor::Larva;
use ln_manager::ln_bridge::settings::Settings as MgrSettings;
use ln_node::settings::Settings as NodeSettings;

pub fn run(ln_conf: MgrSettings, node_conf: NodeSettings) {
    println!("{:#?}", ln_conf);
    println!("{:#?}", node_conf);

    let run_forever = Probe::new(ProbT::Blocking);
    let init_node: Action = Action::new(
        node::gen,
        vec![Arg::MgrConf(ln_conf), Arg::NodeConf(node_conf)],
    );
    run_forever.spawn_task(init_node);
}
