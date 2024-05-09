/// ** This implementation is base on [https://github.com/gfx-rs/wgpu/blob/trunk/xtask/src/util.rs]!**
use std::{io, process::Command};

pub(crate) struct Program {
  pub binary_name: &'static str,
  pub crate_name: &'static str,
}

pub(crate) fn check_all_programs(programs: &[Program]) -> anyhow::Result<()> {
  let mut failed = Vec::new();
  for Program { binary_name, crate_name } in programs {
    let mut cmd = Command::new(binary_name);
    cmd.arg("--help");
    let output = cmd.output();
    match output {
      Ok(_output) => {
        println!("Checking for {binary_name} in PATH: ✅");
      }
      Err(e) if matches!(e.kind(), io::ErrorKind::NotFound) => {
        eprintln!("Checking for {binary_name} in PATH: ❌");
        failed.push(*crate_name);
      }
      Err(e) => {
        eprintln!("Checking for {binary_name} in PATH: ❌");
        panic!("Unknown IO error: {:?}", e);
      }
    }
  }

  if !failed.is_empty() {
    eprintln!("Please install them with: cargo install {}", failed.join(" "));
    anyhow::bail!("Missing programs in PATH");
  }

  Ok(())
}
