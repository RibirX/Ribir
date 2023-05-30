#![feature(test)]

mod counter;
use counter::counter;
use ribir::prelude::*;
use ribir_dev_helper::*;

example_framework!(counter);
