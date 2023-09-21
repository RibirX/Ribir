#![feature(test, return_position_impl_trait_in_trait)]

mod todos;
use ribir::prelude::*;
use ribir_dev_helper::*;
use todos::todos;

example_framework!(todos, wnd_size = Size::new(400., 640.));
