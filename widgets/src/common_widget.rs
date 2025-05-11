use ribir_core::prelude::*;

use crate::layout::HorizontalLine;

#[derive(Template)]
pub struct Leading<T>(T);

#[derive(Template)]
pub struct Trailing<T>(T);

/// `PositionChild` is an enum that can contain a leading child, a trailing
/// child, or a default child.
///
/// It is useful for assisting your widget in
/// gathering a child that is wrapped by `Leading`, `Trailing`, or neither.
#[derive(Template)]
pub enum PositionChild<T> {
  Default(T),
  Leading(Leading<T>),
  Trailing(Trailing<T>),
}

impl<T> PositionChild<T> {
  /// Unwraps the `PositionChild` into its contained value.
  pub fn unwrap(self) -> T {
    match self {
      PositionChild::Default(t) => t,
      PositionChild::Leading(Leading(t)) => t,
      PositionChild::Trailing(Trailing(t)) => t,
    }
  }

  /// Returns `true` if the `PositionChild` is a `Leading` child.
  pub fn is_leading(&self) -> bool { matches!(self, PositionChild::Leading(_)) }

  /// Returns `true` if the `PositionChild` is a `Trailing` child.
  pub fn is_trailing(&self) -> bool { matches!(self, PositionChild::Trailing(_)) }
}

impl<T> Leading<T> {
  pub fn new<K: ?Sized>(child: impl RInto<T, K>) -> Self { Leading(child.r_into()) }

  pub fn unwrap(self) -> T { self.0 }
}

impl<T> Trailing<T> {
  pub fn new<K: ?Sized>(child: impl RInto<T, K>) -> Self { Trailing(child.r_into()) }

  pub fn unwrap(self) -> T { self.0 }
}

/// Composes a widget with a label in horizontal line.
pub fn icon_with_label(icon: Widget, label: Option<PositionChild<TextValue>>) -> Widget {
  let Some(label) = label else { return icon };

  rdl! {
    match label {
     PositionChild::Leading(Leading(text)) => @HorizontalLine {
       @Text { text }
       @ { icon }
     },
     PositionChild::Trailing(Trailing(text)) | PositionChild::Default(text) => @HorizontalLine {
       @ { icon }
       @Text { text }
     }
   }
  }
  .into_widget()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn leading_trailing_declare() {
    reset_test_env!();

    let _leading: Leading<TextValue> = rdl! {
      @Leading { @{ "Leading" } }
    }
    .r_into();

    let _trailing: Trailing<TextValue> = rdl! {
      @Trailing { @{ "Trailing" } }
    }
    .r_into();
  }
}
