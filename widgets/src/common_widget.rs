use ribir_core::prelude::*;

#[derive(Default, Declare, SingleChild)]
pub struct Leading;

#[derive(Default, Declare, SingleChild)]
pub struct Trailing;

pub struct LabelText(pub String);

pub type TrailingText = WidgetPair<Trailing, CowArc<str>>;

pub type LeadingText = WidgetPair<Leading, CowArc<str>>;
