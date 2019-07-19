use ln_cmd::tasks::ln_mgr;
use ln_cmd::tasks::{Action, ProbT, Probe, TaskFn, TaskGen};
use ln_manager::executor::Larva;
use ln_manager::ln_bridge::settings::Settings;

pub fn run(settings: Settings) {
    let run_forever = Probe::new(ProbT::Blocking);
    let test_action: Action = Action::new(ln_mgr::test_gen, false);
    run_forever.spawn_task(test_action);
}
