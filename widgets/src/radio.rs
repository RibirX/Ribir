use ribir_core::prelude::*;

class_names! {
  #[doc = "Class name for the radio button when selected"]
  RADIO_SELECTED,
  #[doc = "Class name for the radio button when unselected"]
  RADIO_UNSELECTED
}

#[derive(Declare)]
pub struct Radio {
  pub checked: bool,
}

impl Compose for Radio {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @ pipe!($this.checked).map(move |checked| {
        fn_widget! {
          let class = match checked {
            true => RADIO_SELECTED,
            false => RADIO_UNSELECTED,
          };
          @ Void { class: class }
        }
      })
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::{Frame, MockMulti, WidgetTester};
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    radio_widget,
    WidgetTester::new(fn_widget! {
      @MockMulti {
        @Radio { checked: false }
        @Radio { checked: true }
      }
    })
    .with_wnd_size(Size::new(150., 80.))
    .with_comparison(0.002)
  );
}
