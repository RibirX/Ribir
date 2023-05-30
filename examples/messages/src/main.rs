#![feature(test)]

mod messages;
use messages::messages;
use ribir::prelude::*;
use ribir_dev_helper::*;

example_framework!(messages, wnd_size = Size::new(480., 960.));
