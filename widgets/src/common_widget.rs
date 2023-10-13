use ribir_core::prelude::*;

#[derive(Default, Declare, PairChild)]
pub struct Leading;

#[derive(Default, Declare, PairChild)]
pub struct Trailing;

pub type TrailingText = Pair<Trailing, CowArc<str>>;

pub type LeadingText = Pair<Leading, CowArc<str>>;
