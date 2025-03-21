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
#[derive(ChildOfCompose)]
pub enum PositionChild<T> {
  Default(T),
  Leading(T),
  Trailing(T),
}

impl<T> PositionChild<T> {
  /// Unwraps the `PositionChild` into its contained value.
  pub fn unwrap(self) -> T {
    match self {
      PositionChild::Default(t) => t,
      PositionChild::Leading(t) => t,
      PositionChild::Trailing(t) => t,
    }
  }

  /// Returns `true` if the `PositionChild` is a `Leading` child.
  pub fn is_leading(&self) -> bool { matches!(self, PositionChild::Leading(_)) }

  /// Returns `true` if the `PositionChild` is a `Trailing` child.
  pub fn is_trailing(&self) -> bool { matches!(self, PositionChild::Trailing(_)) }
}

impl<T> Leading<T> {
  pub fn new<const M: usize>(child: impl IntoChildCompose<T, M>) -> Self {
    Leading(child.into_child_compose())
  }

  pub fn unwrap(self) -> T { self.0 }
}

impl<T> Trailing<T> {
  pub fn new<const M: usize>(child: impl IntoChildCompose<T, M>) -> Self {
    Trailing(child.into_child_compose())
  }

  pub fn unwrap(self) -> T { self.0 }
}

/// Composes a widget with a label in horizontal line.
pub fn icon_with_label(icon: Widget, label: Option<PositionChild<TextInit>>) -> Widget {
  let Some(label) = label else { return icon };

  rdl! {
    match label {
     PositionChild::Leading(text) => @HorizontalLine {
       @Text { text }
       @ { icon }
     },
     PositionChild::Trailing(text) | PositionChild::Default(text) => @HorizontalLine {
       @ { icon }
       @Text { text }
     }
   }
  }
  .into_widget()
}

impl<T> ComposeChildFrom<Leading<T>, 0> for PositionChild<T> {
  fn compose_child_from(from: Leading<T>) -> Self { PositionChild::Leading(from.0) }
}

impl<T> ComposeChildFrom<LeadingBuilder<T>, 0> for PositionChild<T> {
  fn compose_child_from(from: LeadingBuilder<T>) -> Self {
    PositionChild::Leading(from.build_tml().0)
  }
}

impl<T> ComposeChildFrom<Trailing<T>, 0> for PositionChild<T> {
  fn compose_child_from(from: Trailing<T>) -> Self { PositionChild::Trailing(from.0) }
}

impl<T> ComposeChildFrom<TrailingBuilder<T>, 0> for PositionChild<T> {
  fn compose_child_from(from: TrailingBuilder<T>) -> Self {
    PositionChild::Trailing(from.build_tml().0)
  }
}

impl<T> ComposeChildFrom<T, 0> for PositionChild<T> {
  fn compose_child_from(from: T) -> Self { PositionChild::Default(from) }
}

macro_rules! impl_compose_child_from_for_position {
  ($($m:literal),*) => {
    $(
      impl<T, U: ComposeChildFrom<T, $m>> ComposeChildFrom<T, $m> for PositionChild<U> {
        fn compose_child_from(from: T) -> Self {
          PositionChild::Default(U::compose_child_from(from))
        }
      }
    )*
  };
}

impl_compose_child_from_for_position!(1, 2, 3);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn leading_trailing_declare() {
    reset_test_env!();

    let _leading: Leading<TextInit> = rdl! {
      @Leading { @{ "Leading" } }
    }
    .into_child_compose();

    let _trailing: Trailing<TextInit> = rdl! {
      @Trailing { @{ "Trailing" } }
    }
    .into_child_compose();
  }
}
