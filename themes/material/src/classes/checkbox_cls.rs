use md::EDGES_4;
use ribir_core::prelude::*;
use ribir_widgets::checkbox::*;
use svg::named_svgs;

use crate::{md::SIZE_18, *};

const RIPPLE_RADIUS: f32 = 20.;

struct CheckboxColor(Color);

pub(super) fn init(classes: &mut Classes) {
  named_svgs::register(
    UNCHECKED_ICON,
    include_crate_svg!("./icons/unchecked_box.svg", true, false),
  );
  named_svgs::register(CHECKED_ICON, include_crate_svg!("./icons/checked_box.svg", true, false));
  named_svgs::register(
    INDETERMINATE_ICON,
    include_crate_svg!("./icons/indeterminate_box.svg", true, false),
  );

  // Since the only distinction among checkbox states is the color, we consolidate
  // all class logic within `CHECKBOX` and utilize `CheckboxColor` to propagate it
  // across all classes. This approach simplifies the state classes to only handle
  // color settings.
  // Additionally, we ensure that all widgets are reused efficiently.

  classes.insert(CHECKBOX, |w| {
    let foreground = Stateful::new(CheckboxColor(Color::TRANSPARENT));
    let f2 = foreground.clone_writer();
    let hover_layer = HoverLayer::tracked(LayerArea::WidgetCover(md::RADIUS_20));
    let w = FatObj::new(w);
    ripple! {
      radius: RIPPLE_RADIUS,
      center: true,
      margin: EDGES_4,
      foreground: pipe!($foreground.0),

      @ $hover_layer { @ $w {
        margin: EdgeInsets::all(11.),
        clamp: BoxClamp::fixed_size(SIZE_18)
      } }
    }
    .into_widget()
    .attach_data(Box::new(f2))
  });

  classes.insert(CHECKBOX_CHECKED, checkbox_state_impl!(BuildCtx::get().variant_color()));
  classes.insert(CHECKBOX_INDETERMINATE, checkbox_state_impl!(BuildCtx::get().variant_color()));
  classes.insert(
    CHECKBOX_UNCHECKED,
    checkbox_state_impl!(Palette::of(BuildCtx::get()).on_surface_variant()),
  );
}

macro_rules! checkbox_state_impl {
  ($color:expr) => {
    |w| {
      if let Some(mut color) = Provider::write_of::<CheckboxColor>(BuildCtx::get()) {
        color.0 = $color;
      }
      w
    }
  };
}

use checkbox_state_impl;
