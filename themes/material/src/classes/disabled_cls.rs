use ribir_core::prelude::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(DISABLED, |w| {
    color_filter! {
      filter: GRAYSCALE_FILTER,
      opacity: 0.38,
      @ { w }
    }
    .into_widget()
  });
}
