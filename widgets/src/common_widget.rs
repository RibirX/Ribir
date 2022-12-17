use ribir_core::prelude::*;

#[derive(Default, Declare, SingleChild)]
pub struct Leading;

#[derive(Default, Declare, SingleChild)]
pub struct Trailing;

pub struct LabelText(pub CowArc<str>);

pub type TrailingText = WidgetPair<Trailing, CowArc<str>>;

pub type LeadingText = WidgetPair<Leading, CowArc<str>>;

impl LabelText {
  #[inline]
  pub fn new(str: impl Into<CowArc<str>>) -> Self { LabelText(str.into()) }
}
