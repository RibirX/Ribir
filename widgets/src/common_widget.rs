use ribir_core::prelude::*;

#[derive(Default, Declare, SingleChild)]
pub struct Leading;

#[derive(Default, Declare, SingleChild)]
pub struct Trailing;

pub type TrailingText = WidgetPair<Trailing, CowArc<str>>;

pub type LeadingText = WidgetPair<Leading, CowArc<str>>;

impl TmlFlag for Leading {}
impl TmlFlag for Trailing {}
