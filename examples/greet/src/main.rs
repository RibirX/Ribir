#![feature(test)]

mod greet;
use greet::greet;
use ribir::prelude::*;
use ribir_dev_helper::*;

example_framework!(greet, wnd_size = Size::new(640., 400.));
