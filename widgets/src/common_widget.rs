use ribir_core::prelude::{ChildOfCompose, ComposeChildFrom, IntoChildCompose};

#[derive(ChildOfCompose)]
pub struct Leading<T>(T);

#[derive(ChildOfCompose)]
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

impl<T> ComposeChildFrom<Leading<T>, 0> for PositionChild<T> {
  fn compose_child_from(from: Leading<T>) -> Self { PositionChild::Leading(from.0) }
}

impl<T> ComposeChildFrom<Trailing<T>, 0> for PositionChild<T> {
  fn compose_child_from(from: Trailing<T>) -> Self { PositionChild::Trailing(from.0) }
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
