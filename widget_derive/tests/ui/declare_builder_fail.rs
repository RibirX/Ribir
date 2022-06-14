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
  #[declare(custom_convert)]
  x: Option<i32>,
}

impl ConverterBuilder {
  #[inline]
  pub fn x_convert<M, X: Into<StripedOption<i32, M>>>(x: X) -> Option<i32> { x.into().value }
}
fn main() {}
