use ribir_core::prelude::*;

#[derive(Default, Declare, Declare2, SingleChild)]
pub struct Leading;

#[derive(Default, Declare, Declare2, SingleChild)]
pub struct Trailing;

pub type TrailingText = SinglePair<Trailing, CowArc<str>>;

pub type LeadingText = SinglePair<Leading, CowArc<str>>;

impl TmlHolder for Leading {}
impl TmlHolder for Trailing {}
