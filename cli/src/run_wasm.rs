use std::{
  path::{Path, PathBuf},
  str::FromStr,
  thread,
  time::Duration,
};

use anyhow::Result;
use clap::{command, CommandFactory, FromArgMatches, Parser};
use notify_debouncer_mini::{new_debouncer, notify::*, DebounceEventResult, Debouncer};

use crate::{
  program_check::{check_all_programs, Program},
  CliCommand,
};

const WATCH_DEBOUNCE_GAP: Duration = Duration::from_secs(2);

pub fn run_wasm() -> Box<dyn CliCommand> { Box::new(RunWasm {}) }

struct RunWasm {}
#[derive(Parser, Debug, Clone)]
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

impl Wasm {
  fn root_path(&self) -> Result<PathBuf> {
    let root = PathBuf::from_str(env!("CARGO_WORKSPACE_DIR"))?;
    Ok(root)
  }

  fn out_dir(&self) -> PathBuf {
    let out_dir = self
      .out_dir
      .clone()
      .unwrap_or(PathBuf::from("./target/wasm"));

    if out_dir.is_relative() {
      let root_path = PathBuf::from_str(env!("CARGO_WORKSPACE_DIR")).unwrap();
      root_path.clone().join(&out_dir)
    } else {
      out_dir.clone()
    }
  }

  fn output_name(&self) -> String {
    self
      .name
      .clone()
      .unwrap_or("web_wasm".to_string())
  }

  fn auto_rebuild(&self) -> Debouncer<RecommendedWatcher> {
    let root_path = self.root_path().unwrap();
    let ignore_file = root_path.join(".gitignore");
    let this = self.clone();
    let mut debouncer =
      new_debouncer(WATCH_DEBOUNCE_GAP, move |res: DebounceEventResult| match res {
        Ok(events) => {
          let ignore = gitignore::File::new(Path::new(&ignore_file));
          let need_rebuild = events.iter().any(|e| {
            if let Ok(ignore) = &ignore {
              return !ignore.is_excluded(&e.path).unwrap_or(false);
            }
            true
          });
          if need_rebuild {
            let _ = this.wasm_build();
          }
        }
        Err(e) => println!("Error {:?}", e),
      })
      .unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    debouncer
      .watcher()
      .watch(&root_path, RecursiveMode::Recursive)
      .unwrap();
    debouncer
  }

  fn wasm_build(&self) -> Result<()> {
    let shell = xshell::Shell::new()?;
    let root_path = self.root_path()?;
    let output = self.out_dir();
    let release_flg = if self.release { Some("--release") } else { None };
    let package = self.package.clone();

    xshell::cmd!(
      shell,
      "cargo build -p {package} --lib  {release_flg...} --target wasm32-unknown-unknown"
    )
    .quiet()
    .run()?;

    shell.change_dir(env!("CARGO_WORKSPACE_DIR"));
    let target_path = if self.release {
      "target/wasm32-unknown-unknown/release"
    } else {
      "target/wasm32-unknown-unknown/debug"
    };

    let package = self.package.clone();
    let name = self.output_name();
    xshell::cmd!(
      shell,
      "wasm-bindgen {target_path}/{package}.wasm --target web
    --no-typescript --out-dir {output} --out-name {name}"
    )
    .quiet()
    .run()?;

    if let Some(mut path) = self.template.clone() {
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
    Ok(())
  }

  fn server(&self) -> Result<()> {
    let root_path = self.root_path()?;
    let _watcher = self.auto_rebuild();
    let out_dir = self.out_dir();
    let handle = thread::Builder::new().spawn(move || {
      let shell = xshell::Shell::new().unwrap();
      shell.change_dir(root_path);
      let _ = xshell::cmd!(
        shell,
        "simple-http-server {out_dir} -c wasm,html,js -i --coep --coop
          --nocache"
      )
      .quiet()
      .run();
    })?;

    handle.join().unwrap();
    Ok(())
  }
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

    args.wasm_build()?;
    if !args.no_server {
      args.server()?;
    }

    Ok(())
  }
}
