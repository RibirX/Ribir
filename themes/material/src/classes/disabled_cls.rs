use ribir_core::prelude::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(DISABLED, |w| {
    fn_widget! {
      @ColorFilter {
        filter: GRAYSCALE_FILTER,
        opacity: 0.8,
        @ { w }
      }
    }
    .into_widget()
  });
}
