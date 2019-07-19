use ln_cmd::tasks::{Probe, ProbT, TaskFn, TaskGen, Action};

fn test_task() -> Result<(), String> {
    println!("hello, test");
    Ok(())
}

pub fn test_gen() -> Box<TaskFn> {
    Box::new(test_task)
}
