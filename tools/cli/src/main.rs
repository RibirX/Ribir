mod bundle;
mod mcp;
mod program_check;
mod run_wasm;
mod util;

use anyhow::Result;
use clap::ArgMatches;
use run_wasm::run_wasm;
use tracing::Level;
use tracing_subscriber::{
  EnvFilter,
  field::Visit,
  fmt::{FormatEvent, FormatFields, format::Writer},
  prelude::*,
};

use crate::{bundle::bundle, mcp::mcp};

trait CliCommand {
  fn name(&self) -> &str;
  fn command(&self) -> clap::Command;
  fn exec(&self, args: &ArgMatches) -> Result<()>;
}

struct CliFormatter;

impl<S, N> FormatEvent<S, N> for CliFormatter
where
  S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
  N: for<'a> FormatFields<'a> + 'static,
{
  fn format_event(
    &self, ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>, mut writer: Writer<'_>,
    event: &tracing::Event<'_>,
  ) -> std::fmt::Result {
    let mut visitor = ActionVisitor::default();
    event.record(&mut visitor);

    let is_command_output =
      visitor.action.as_deref() == Some("stdout") || visitor.action.as_deref() == Some("stderr");
    let level = *event.metadata().level();

    if let Some(action) = visitor.action {
      if !is_command_output {
        write!(writer, "\x1b[1;32m{:>12}\x1b[0m ", action)?;
      }
    } else {
      let level_str = format!("{}", level);
      let color = match level {
        Level::ERROR => "\x1b[1;31m",
        Level::WARN => "\x1b[1;33m",
        Level::INFO => "\x1b[1;32m",
        Level::DEBUG => "\x1b[1;34m",
        Level::TRACE => "\x1b[1;35m",
      };
      write!(writer, "{}{:>12}\x1b[0m ", color, level_str)?;
    }

    if !is_command_output && level == Level::DEBUG {
      write!(writer, "\x1b[30m[{}]\x1b[0m ", event.metadata().target())?;
    }

    ctx.format_fields(writer.by_ref(), event)?;
    writeln!(writer)
  }
}

#[derive(Default)]
struct ActionVisitor {
  action: Option<String>,
}

impl Visit for ActionVisitor {
  fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
    if field.name() == "action" {
      self.action = Some(value.to_string());
    }
  }

  fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
    if field.name() == "action" && self.action.is_none() {
      self.action = Some(format!("{:?}", value));
    }
  }
}

fn init_log(log_level: Level) {
  let filter = EnvFilter::from_default_env().add_directive(log_level.into());

  tracing_subscriber::registry()
    .with(filter)
    .with(tracing_subscriber::fmt::layer().event_format(CliFormatter))
    .init();
}

fn main() {
  let mut cli = clap::Command::new("cli").bin_name("cli");

  let commands = [run_wasm(), bundle(), mcp()];

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
        init_log(Level::INFO);
      } else {
        init_log(Level::ERROR);
      }

      let res = cmd.exec(matches);
      if let Err(err) = res {
        tracing::error!("{}", err);
        std::process::exit(1);
      }
    }
  }
}
