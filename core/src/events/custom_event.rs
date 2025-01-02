use std::any::Any;

use crate::prelude::*;

/// A custom event.
///
/// you can bubble the custom event to the ancestor widgets, by call
/// [`Window.bubble_custom_event`].
///
/// To listen to the custom event, you can register the event handler to a
/// specific custom event by [`on_custom_concrete_event`], or register a handler
/// to [`RawCustomEvent`] and downcast it to the specific type as you want by
/// [`on_custom_event`].
pub struct CustomEvent<E: ?Sized> {
  pub(crate) common: CommonEvent,
  pub(crate) data: Box<dyn Any>,
  _marker: std::marker::PhantomData<E>,
}

pub type RawCustomEvent = CustomEvent<dyn Any>;

pub fn new_custom_event(common: CommonEvent, data: Box<dyn Any>) -> RawCustomEvent {
  CustomEvent { common, data, _marker: std::marker::PhantomData }
}

impl<E: ?Sized> std::ops::Deref for CustomEvent<E> {
  type Target = CommonEvent;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.common }
}

impl<E: ?Sized> std::ops::DerefMut for CustomEvent<E> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
}

impl<E: Sized + 'static> CustomEvent<E> {
  pub fn data(&self) -> &E { self.data.downcast_ref::<E>().unwrap() }

  pub fn data_mut(&mut self) -> &mut E { self.data.downcast_mut::<E>().unwrap() }
}

impl<E: ?Sized> AsRef<ProviderCtx> for CustomEvent<E> {
  fn as_ref(&self) -> &ProviderCtx { self.common.as_ref() }
}

impl<E: ?Sized> AsMut<ProviderCtx> for CustomEvent<E> {
  fn as_mut(&mut self) -> &mut ProviderCtx { self.common.as_mut() }
}

impl RawCustomEvent {
  /// Downcast the event reference to a specific type CustomEvent<E> that cast
  /// data to `E`, return `None` if the downcast fails
  pub fn downcast_ref<E: 'static>(&self) -> Option<&CustomEvent<E>> {
    if self.data.downcast_ref::<E>().is_some() {
      Some(unsafe { &*(self as *const CustomEvent<dyn Any> as *const CustomEvent<E>) })
    } else {
      None
    }
  }

  /// Downcast the event mut reference to a specific type CustomEvent<E> that
  /// cast data to `E`, return `None` if the downcast fails
  pub fn downcast_mut<E: 'static>(&mut self) -> Option<&mut CustomEvent<E>> {
    if self.data.downcast_mut::<E>().is_some() {
      Some(unsafe { &mut *(self as *mut CustomEvent<dyn Any> as *mut CustomEvent<E>) })
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{prelude::*, reset_test_env, test_helper::*};
  #[test]
  fn custom_event() {
    reset_test_env!();
    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    struct MyCustomData(i32);
    type MyCustomEvent = CustomEvent<MyCustomData>;

    let (self_data, w_self_data) = split_value(None);
    let (parent_data, w_parent_data) = split_value(None);
    let w = fn_widget! {
      @MockBox {
        size: Size::new(100., 100.),
        on_custom_event: move |e: &mut RawCustomEvent| {
          if let Some(e) = e.downcast_mut() {
            *$w_parent_data.write() = Some(*e.data());
          }
        },
        @MockBox {
          size: Size::new(50., 50.),
          on_mounted: move |e| {
            e.window().bubble_custom_event(e.widget_id(), MyCustomData(1));
          },
          on_custom_concrete_event: move |e: &mut MyCustomEvent| {
            *$w_self_data.write() = Some(*e.data());
          }
        }
      }
    };
    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    assert_eq!(*self_data.read(), Some(MyCustomData(1)));
    assert_eq!(*parent_data.read(), Some(MyCustomData(1)));
  }
}
