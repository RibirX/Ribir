#![feature(trivial_bounds)]

use ribir::prelude::*;

#[derive(Declare)]
struct ReservedNames {
  margin: i32,
}

#[derive(Declare)]
struct RenameReservedNames {
  #[rename = "margin_data"]
  margin: i32,
}

fn main() {}
