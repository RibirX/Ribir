mod program_check;
mod run_wasm;

use anyhow::Result;
use clap::ArgMatches;
use run_wasm::run_wasm;

trait CliCommand {
  fn name(&self) -> &str;
  fn command(&self) -> clap::Command;
  fn exec(&self, args: &ArgMatches) -> Result<()>;
}

fn main() {
  let mut cli = clap::Command::new("cli").bin_name("cli");

  let commands = [run_wasm()];

  for cmd in &commands {
    cli = cli.subcommand(cmd.command());
  }
  let matches = cli.get_matches();

  if let Some((sub_cmd, matches)) = matches.subcommand() {
    if let Some(cmd) = commands.iter().find(|cmd| cmd.name() == sub_cmd) {
      let _ = cmd.exec(matches);
    }
  }
}
