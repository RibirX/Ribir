#![feature(test)]

mod storybook;
use ribir::prelude::*;
use ribir_dev_helper::*;
use storybook::storybook;

example_framework!(storybook, wnd_size = Size::new(1024., 768.));
