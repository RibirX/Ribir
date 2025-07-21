mod bundle;
mod program_check;
mod run_wasm;
mod util;
use std::io::Write;

use anyhow::Result;
use clap::ArgMatches;
use env_logger::{
  Builder,
  fmt::style::{AnsiColor, Style},
};
use log::Level;
use run_wasm::run_wasm;

use crate::bundle::bundle;

trait CliCommand {
  fn name(&self) -> &str;
  fn command(&self) -> clap::Command;
  fn exec(&self, args: &ArgMatches) -> Result<()>;
}

fn init_log(log_level: Level) {
  let mut builder = Builder::from_default_env();
  let _ = builder
    .format_indent(Some(12))
    .filter_level(log_level.to_level_filter())
    .format(|f, record| {
      let mut is_command_output = false;
      if let Some(action) = record.key_values().get("action".into()) {
        let action = action.to_cow_str().unwrap();
        is_command_output = action == "stdout" || action == "stderr";
        if !is_command_output {
          let style = Style::new()
            .fg_color(Some(AnsiColor::Green.into()))
            .bold();

          write!(f, "{style}{action:>12}{style:#} ")?;
        }
      } else {
        let style = f.default_level_style(record.level()).bold();
        write!(f, "{style}{:>12}{style:#} ", record.level())?;
      }

      if !is_command_output && log::log_enabled!(Level::Debug) {
        let style = Style::new().fg_color(Some(AnsiColor::Black.into()));

        write!(f, "[{style}{}{style:#}] ", record.target())?;
      }

      writeln!(f, "{}", record.args())
    })
    .try_init();
}

fn main() {
  let mut cli = clap::Command::new("cli").bin_name("cli");

  let commands = [run_wasm(), bundle()];

  for cmd in &commands {
    cli = cli.subcommand(cmd.command());
  }
  let matches = cli.get_matches();

  if let Some((sub_cmd, matches)) = matches.subcommand() {
    if let Some(cmd) = commands.iter().find(|cmd| cmd.name() == sub_cmd) {
      if matches
        .try_get_one::<bool>("verbose")
        .is_ok_and(|v| v.is_some_and(|v| *v))
      {
        init_log(Level::Info);
      } else {
        init_log(Level::Error);
      }

      let res = cmd.exec(matches);
      if let Err(err) = res {
        log::error!("{}", err);
      }
    }
  }
}
