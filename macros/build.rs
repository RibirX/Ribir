use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use builtin::builtin;

include!("./src/builtin_fields_list.rs");

fn main() -> std::io::Result<()> {
  let out_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("../docs/declare_builtin_fields.md");
  let mut f = fs::File::create(&dest_path)?;
  f.write_all(b"# Full builtin fields list \n\n")?;

  for w in WIDGETS.iter() {
    for field in w.fields.iter() {
      f.write_all(
        format!(
          "- {} : [`{}`] \n \t - {}\n",
          field.name, field.ty, field.doc
        )
        .as_bytes(),
      )?;
    }
  }

  for w in WIDGETS.iter() {
    for field in w.fields.iter() {
      let ty_name = field.ty;
      f.write_all(format!("\n[`{ty_name}`]: prelude::{ty_name}\n",).as_bytes())?;
    }
  }

  println!("cargo:rerun-if-changed=build.rs");
  Ok(())
}
