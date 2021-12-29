#![feature(trivial_bounds)]

use ribir::prelude::*;

#[derive(Declare)]
struct ReservedNames {
  margin: i32,
}

#[derive(Declare)]
struct RenameReservedNames {
  #[declare(rename = "margin_data")]
  margin: i32,
}

#[derive(Declare)]
struct Converter {
  #[declare(convert(into, some))]
  x: Option<i32>,
}

fn main() {}
