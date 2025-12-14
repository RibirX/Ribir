use ribir_core::prelude::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    DISABLED,
    style_class! {
      filter: Filter::grayscale(1.),
      opacity: 0.38,
    },
  );
}
