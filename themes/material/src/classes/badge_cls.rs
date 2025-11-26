use ribir_core::prelude::*;
use ribir_widgets::prelude::{BADGE_LARGE, BADGE_SMALL, BadgeColor};

fn get_badge_color() -> VariantMap<BadgeColor, impl Fn(&BadgeColor) -> Color + Clone> {
  Variant::new_or_else(BuildCtx::get(), || BadgeColor(Palette::of(BuildCtx::get()).error()))
    .map(|c| c.0)
}

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    BADGE_SMALL,
    style_class! {
      clamp: BoxClamp::fixed_size(Size::new(6., 6.)),
      radius: Radius::all(3.),
      padding: EdgeInsets::all(0.),
      background: get_badge_color(),
    },
  );

  classes.insert(BADGE_LARGE, move |w| {
    fn_widget! {
      let color = get_badge_color();
      @FatObj {
        clamp: BoxClamp::min_width(16.).with_min_height(16.),
        radius: Radius::all(8.),
        padding: EdgeInsets::horizontal(4.),
        background: color.clone(),
        foreground: color.on_this_color(BuildCtx::get()),
        text_style: TypographyTheme::of(BuildCtx::get()).label_small.text.clone(),
        @ { w }
      }
    }
    .into_widget()
  });
}
