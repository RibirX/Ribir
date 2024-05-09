use std::{path::PathBuf, str::FromStr};

use anyhow::Result;
use clap::{command, CommandFactory, FromArgMatches, Parser};

use crate::{
  program_check::{check_all_programs, Program},
  CliCommand,
};

pub fn run_wasm() -> Box<dyn CliCommand> { Box::new(RunWasm {}) }

struct RunWasm {}

#[derive(Parser, Debug)]
#[command(name = "run-wasm")]
/// run as web wasm
struct Wasm {
  /// Package of which to run
  #[arg(short, long)]
  package: String,

  /// Name of output, default to web_wasm
  #[arg(short, long)]
  name: Option<String>,

  /// Direction path to output, default to target/wasm
  #[arg(short, long)]
  out_dir: Option<PathBuf>,

  /// Build release, default build to debug
  #[arg(short, long)]
  release: bool,

  /// Just build the wasm files, don't serve them
  #[arg(long, name = "no-server")]
  no_server: bool,

  /// Template files need to copy to Output dir
  #[arg(short, long)]
  template: Option<PathBuf>,
}

impl CliCommand for RunWasm {
  fn name(&self) -> &str { "run-wasm" }

  fn command(&self) -> clap::Command { Wasm::command() }

  fn exec(&self, args: &clap::ArgMatches) -> Result<()> {
    let args = Wasm::from_arg_matches(args)?;

    let mut dependencies =
      vec![Program { crate_name: "wasm-bindgen-cli", binary_name: "wasm-bindgen" }];

    if !args.no_server {
      dependencies
        .push(Program { crate_name: "simple-http-server", binary_name: "simple-http-server" });
    }
    check_all_programs(&dependencies)?;

    let root_path = PathBuf::from_str(env!("CARGO_WORKSPACE_DIR"))?;
    let package = args.package;
    let name = args.name.unwrap_or("web_wasm".to_string());

    let out_dir = args
      .out_dir
      .unwrap_or(PathBuf::from("./target/wasm"));
    let output =
      if out_dir.is_relative() { root_path.clone().join(&out_dir) } else { out_dir.clone() };

    let shell = xshell::Shell::new()?;
    let release_flg = if args.release { Some("--release") } else { None };

    xshell::cmd!(
      shell,
      "cargo build -p {package} --lib  {release_flg...} --target wasm32-unknown-unknown"
    )
    .quiet()
    .run()?;

    shell.change_dir(env!("CARGO_WORKSPACE_DIR"));
    let target_path = if args.release {
      "target/wasm32-unknown-unknown/release"
    } else {
      "target/wasm32-unknown-unknown/debug"
    };

    xshell::cmd!(
      shell,
      "wasm-bindgen {target_path}/{package}.wasm --target web
      --no-typescript --out-dir {output} --out-name {name}"
    )
    .quiet()
    .run()?;

    if let Some(mut path) = args.template.clone() {
      if path.is_relative() {
        path = root_path.clone().join(path);
      }
      if path.is_dir() {
        fs_extra::dir::copy(
          &path,
          &output,
          &fs_extra::dir::CopyOptions::new()
            .overwrite(true)
            .content_only(true),
        )?;
      } else {
        let file_name = output.clone().join(path.file_name().unwrap());
        fs_extra::file::copy(
          &path,
          file_name,
          &fs_extra::file::CopyOptions::new().overwrite(true),
        )?;
      }
    }

    if !args.no_server {
      shell.change_dir(root_path);
      xshell::cmd!(
        shell,
        "simple-http-server {out_dir} -c wasm,html,js -i --coep --coop --nocache"
      )
      .quiet()
      .run()?;
    }

    Ok(())
  }
}
