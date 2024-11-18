use ribir_core::prelude::*;
use ribir_widgets::icon::ICON;

use crate::md;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(ICON, |w| {
    let font_face = Theme::of(BuildCtx::get()).icon_font.clone();

    FatObj::new(w)
      .clamp(BoxClamp::fixed_size(md::SIZE_24))
      .text_style(TextStyle {
        line_height: 24.,
        font_size: 24.,
        letter_space: 0.,
        font_face,
        overflow: Overflow::Clip,
      })
      .into_widget()
  });
}
