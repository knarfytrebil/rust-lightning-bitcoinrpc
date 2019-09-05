#[macro_use]
extern crate clap;
use clap::App;
mod commands;

fn main() {
    let yaml = load_yaml!("conf/en_US.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let commands = vec!["info", "invoice", "channel", "peer"];

    commands.into_iter().for_each(
        |command| if let Some(sub_matches) =
            matches.subcommand_matches(command)
        {
            let sub_commands = vec![ "node", "addresses", "create", "pay", "kill", "killall", "list", "connect",];
            sub_commands.into_iter().for_each(|sub_command| {
                if sub_matches.is_present(sub_command) {
                    commands::react(command, sub_command, &matches, sub_matches);
                }
            });
        },
    );
}
