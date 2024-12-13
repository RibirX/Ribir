use ribir_core::prelude::{ChildOfCompose, ComposeChildFrom, IntoChildCompose};

#[derive(ChildOfCompose)]
pub struct Leading<T>(T);

#[derive(ChildOfCompose)]
pub struct Trailing<T>(T);

#[derive(ChildOfCompose)]
pub enum PositionChild<T> {
  Default(T),
  Leading(T),
  Trailing(T),
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
