#![feature(test)]

mod todos;
use ribir::prelude::*;
use ribir_dev_helper::*;
use todos::todos;

example_framework!(todos, wnd_size = Size::new(400., 640.));
