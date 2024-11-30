use ribir_core::{
  prelude::IntoChild,
  widget::{COMPOSE, FN, RENDER},
};

pub struct Leading<T>(pub T);

pub struct Trailing<T>(pub T);

macro_rules! impl_into_widget_child {
  ($($marker:ident),*) => {
    $(
      impl<U, T> IntoChild<Leading<U>, $marker> for Leading<T>
      where
          T: IntoChild<U, $marker>,
        {
          fn into_child(self) -> Leading<U> {
            Leading(self.0.into_child())
          }
        }

      impl<U, T> IntoChild<Trailing<U>, $marker> for Trailing<T>
      where
          T: IntoChild<U, $marker>,
        {
          fn into_child(self) -> Trailing<U> {
            Trailing(self.0.into_child())
          }
        }
    )*
  };
}

impl_into_widget_child!(COMPOSE, RENDER, FN);
