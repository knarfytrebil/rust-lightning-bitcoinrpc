#[macro_use]
extern crate clap;
use clap::{App, AppSettings};
mod commands;

fn main() {
    let yaml = load_yaml!("conf/en_US.yml");
    let matches = App::from_yaml(yaml)
        .setting(AppSettings::ColoredHelp)
        .get_matches();

    let commands = vec!["get", "connect", "list", "channel"];

    commands.into_iter().for_each(|command| {
        if matches.is_present(command) {
            commands::react(command, &matches);
        }
    });
}
