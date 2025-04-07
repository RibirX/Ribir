use ribir_core::prelude::*;

/// A provider used to hint widgets in the subtree to show the focus indicator
/// or not.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ShowFocusRing(pub bool);

///  As default, the focused widget by keyboard will show the indicator
/// in web platform, and a focus layer in the native platform.
#[derive(Declare, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FocusIndicator {}


impl FocusIndicator {
  pub fn new(child: FatObj<Child>) -> 
}

pub struct FocusRing {}
