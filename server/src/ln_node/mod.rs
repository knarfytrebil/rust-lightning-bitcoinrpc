pub mod settings;

use ln_cmd::tasks::node;
use ln_cmd::tasks::{Action, ProbT, Probe, TaskFn, TaskGen};
use ln_manager::executor::Larva;
use ln_manager::ln_bridge::settings::Settings;

pub fn run(settings: Settings) {
    println!("{:#?}", settings);
    let run_forever = Probe::new(ProbT::Blocking);
    let init_node: Action = Action::new(node::gen, false);
    run_forever.spawn_task(init_node);
}
