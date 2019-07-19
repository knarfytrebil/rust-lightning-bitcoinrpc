use ln_manager::executor::Larva;
use ln_cmd::tasks::{Probe, ProbT, TaskFn, TaskGen, Action};
use ln_cmd::tasks::{ln_mgr};

pub fn run() {
    let run_forever = Probe::new(ProbT::Blocking);
    let test_action: Action = Action::new(ln_mgr::test_gen, false);
    run_forever.spawn_task(test_action);
}
