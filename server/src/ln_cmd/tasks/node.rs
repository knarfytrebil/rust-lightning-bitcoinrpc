use ln_cmd::tasks::{Probe, ProbT, TaskFn, TaskGen, Action};

fn node() -> Result<(), String> {
    println!("hello, test");
    Ok(())
}

pub fn gen() -> Box<TaskFn> {
    Box::new(node)
}
