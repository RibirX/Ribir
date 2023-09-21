#![feature(test, return_position_impl_trait_in_trait)]

mod messages;
use messages::messages;
use ribir::prelude::*;
use ribir_dev_helper::*;

example_framework!(messages);
