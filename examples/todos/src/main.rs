#![cfg_attr(test, feature(test))]

mod todos;
use ribir::prelude::*;
use ribir_dev_helper::*;
mod ui;
use ui::todos;

example_framework!(todos, wnd_size = Size::new(400., 640.));
