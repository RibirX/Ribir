use md::EDGES_4;
use ribir_core::prelude::*;
use ribir_widgets::checkbox::*;
use svg::named_svgs;

use crate::*;

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
    let hover_layer = HoverLayer::tracked(LayerArea::WidgetCover(md::RADIUS_20));
    let w = FatObj::new(w);
    ripple! {
      radius: 20.,
      center: true,
      margin: EDGES_4,
      @ $hover_layer { @ $w {
        text_line_height: 18.,
        margin: EdgeInsets::all(11.),
      } }
    }
    .into_widget()
  });

  classes.insert(CHECKBOX_CHECKED, style_class! {
    foreground: BuildCtx::get().variant_color()
  });
  classes.insert(CHECKBOX_INDETERMINATE, style_class! {
    foreground: BuildCtx::get().variant_color()
  });
  classes.insert(CHECKBOX_UNCHECKED, style_class! {
    foreground: Palette::of(BuildCtx::get()).on_surface_variant()
  });
}
