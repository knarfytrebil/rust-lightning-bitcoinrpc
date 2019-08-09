#[macro_use]
extern crate clap;
use clap::App;
mod commands;

fn main() {
    // Load Command Mappings
    let yaml = load_yaml!("conf/en_US.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let commands = vec!["get", "connect", "list"];

    commands.into_iter().for_each(|command| {
        if matches.is_present(command) {
            commands::react(command, &matches);
        }
    });
}
