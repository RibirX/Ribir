use ribir_core::{named_style_class, prelude::*};
use ribir_widgets::prelude::*;

use crate::*;

const BTN_40_CLAMP: BoxClamp = BoxClamp::min_width(40.).with_fixed_height(40.);

pub(super) fn init(classes: &mut Classes) {
  button_init(classes);
  filled_button_init(classes);
  text_button_init(classes);
  fab_init(classes);
  mini_fab_init(classes);
  large_fab_init(classes);
}

named_style_class!(common_btn => {
  padding: md::EDGES_HOR_16,
  text_style: btn_label_style(18.)
});
named_style_class!(common_btn_label => { padding: md::EDGES_HOR_8 });
named_style_class!(common_icon_only => { text_line_height: 24. });
named_style_class!(common_label_only => {
  padding: md::EDGES_HOR_24,
  text_style: btn_label_style(40.)
});

fn text_button_init(classes: &mut Classes) {
  fn interactive(w: Widget) -> Widget {
    FatObj::new(base_interactive(w, md::RADIUS_20))
      .foreground(BuildCtx::color())
      .clamp(BTN_40_CLAMP)
      .into_widget()
  }

  classes.insert(TEXT_BTN, multi_class![
    style_class! { padding: md::EDGES_HOR_12, text_style: btn_label_style(18.) },
    interactive
  ]);
  classes.insert(TEXT_BTN_LABEL, style_class! { margin: md::EDGES_HOR_4 });
  classes.insert(TEXT_BTN_LEADING_ICON, style_class! { margin: md::EDGES_RIGHT_4 });
  classes.insert(TEXT_BTN_TRAILING_ICON, style_class! { margin: md::EDGES_LEFT_4 });

  classes.insert(TEXT_BTN_LABEL_ONLY, multi_class![
    style_class! { padding: md::EDGES_HOR_12, text_style: btn_label_style(40.) },
    interactive
  ]);
  classes.insert(TEXT_BTN_ICON_ONLY, multi_class![common_icon_only, interactive]);
}

fn filled_button_init(classes: &mut Classes) {
  fn filled_interactive(w: Widget) -> Widget {
    let color = BuildCtx::color();
    let w = FatObj::new(w)
      .background(color)
      .border_radius(md::RADIUS_20)
      .clamp(BTN_40_CLAMP)
      .into_widget();

    FatObj::new(base_interactive(w, md::RADIUS_20))
      .foreground(BuildCtx::color().on_this_color(BuildCtx::get()))
      .into_widget()
  }

  classes.insert(FILLED_BTN, multi_class![common_btn, filled_interactive]);
  classes.insert(FILLED_BTN_LABEL, common_btn_label);
  classes.insert(FILLED_BTN_LEADING_ICON, empty_cls);
  classes.insert(FILLED_BTN_TRAILING_ICON, empty_cls);

  classes.insert(FILLED_BTN_LABEL_ONLY, multi_class![common_label_only, filled_interactive]);
  classes.insert(FILLED_BTN_ICON_ONLY, multi_class![common_icon_only, filled_interactive]);
}

fn button_init(classes: &mut Classes) {
  fn outlined_interactive(w: Widget) -> Widget {
    let outline = Palette::of(BuildCtx::get()).outline();
    let w = FatObj::new(w)
      .border(Border::all(BorderSide { color: outline.into(), width: 1. }))
      .border_radius(md::RADIUS_20)
      .clamp(BTN_40_CLAMP)
      .into_widget();

    FatObj::new(base_interactive(w, md::RADIUS_20))
      .foreground(BuildCtx::color())
      .into_widget()
  }

  classes.insert(BUTTON, multi_class![common_btn, outlined_interactive]);
  classes.insert(BTN_LABEL, common_btn_label);
  classes.insert(BTN_LEADING_ICON, empty_cls);
  classes.insert(BTN_TRAILING_ICON, empty_cls);

  classes.insert(BTN_LABEL_ONLY, multi_class![common_label_only, outlined_interactive]);
  classes.insert(BTN_ICON_ONLY, multi_class![common_icon_only, outlined_interactive]);
}

fn fab_common_interactive(w: Widget, radius: Radius, btn_clamp: BoxClamp) -> Widget {
  let color = BuildCtx::color();

  let w = FatObj::new(w)
    .background(
      color
        .clone()
        .into_container_color(BuildCtx::get()),
    )
    .clamp(btn_clamp)
    .border_radius(radius);
  FatObj::new(base_interactive(w.into_widget(), radius))
    .foreground(color.on_this_container_color(BuildCtx::get()))
    .into_widget()
}

fn fab_init(classes: &mut Classes) {
  const BTN_HEIGHT: f32 = 56.;
  fn fab_interactive(w: Widget) -> Widget {
    let clamp = BoxClamp::min_width(BTN_HEIGHT).with_fixed_height(BTN_HEIGHT);
    fab_common_interactive(w, md::RADIUS_16, clamp)
  }

  classes.insert(FAB_ICON_ONLY, multi_class![common_icon_only, fab_interactive]);
  classes.insert(FAB_LABEL_ONLY, multi_class![
    style_class! { padding: md::EDGES_HOR_24, text_style: btn_label_style(BTN_HEIGHT) },
    fab_interactive
  ]);

  classes.insert(FAB, multi_class![
    style_class! {
      padding: md::EDGES_HOR_16,
      text_style: btn_label_style(24.)
    },
    fab_interactive
  ]);
  classes.insert(FAB_LEADING_ICON, empty_cls);
  classes.insert(FAB_TRAILING_ICON, empty_cls);
  classes.insert(FAB_LABEL, common_btn_label);
}

fn mini_fab_init(classes: &mut Classes) {
  fn fab_interactive(w: Widget) -> Widget { fab_common_interactive(w, md::RADIUS_12, BTN_40_CLAMP) }

  classes.insert(MINI_FAB_ICON_ONLY, multi_class![common_icon_only, fab_interactive]);
  classes.insert(MINI_FAB_LABEL_ONLY, multi_class![common_label_only, fab_interactive]);
  classes.insert(MINI_FAB, multi_class![common_btn, fab_interactive]);
  classes.insert(MINI_FAB_LEADING_ICON, empty_cls);
  classes.insert(MINI_FAB_TRAILING_ICON, empty_cls);
  classes.insert(MINI_FAB_LABEL, common_btn_label);
}

fn large_fab_init(classes: &mut Classes) {
  const ICON_SIZE: f32 = 36.;
  const BTN_HEIGHT: f32 = 96.;

  fn label_style(line_height: f32) -> TextStyle {
    let text_theme = TypographyTheme::of(BuildCtx::get());
    let mut text_style = text_theme.title_large.text.clone();
    text_style.line_height = line_height;
    text_style
  }

  fn fab_interactive(w: Widget) -> Widget {
    let clamp = BoxClamp::min_width(BTN_HEIGHT).with_fixed_height(BTN_HEIGHT);
    fab_common_interactive(w, Radius::all(28.), clamp)
  }

  classes.insert(LARGE_FAB_ICON_ONLY, multi_class![
    style_class! { text_line_height: ICON_SIZE },
    fab_interactive
  ]);
  classes.insert(LARGE_FAB_LABEL_ONLY, multi_class![
    style_class! {
      text_style: label_style(BTN_HEIGHT),
      padding: md::EDGES_HOR_48,
    },
    fab_interactive
  ]);
  classes.insert(LARGE_FAB, multi_class![
    style_class! { padding: md::EDGES_HOR_32,
      text_style: label_style(ICON_SIZE)
    },
    fab_interactive
  ]);
  classes.insert(LARGE_FAB_LEADING_ICON, empty_cls);
  classes.insert(LARGE_FAB_TRAILING_ICON, empty_cls);
  classes.insert(LARGE_FAB_LABEL, style_class! { padding: md::EDGES_HOR_16 });
}

fn btn_label_style(line_height: f32) -> TextStyle {
  let text_theme = TypographyTheme::of(BuildCtx::get());
  let mut text_style = text_theme.label_large.text.clone();
  text_style.line_height = line_height;
  text_style
}

fn base_interactive(w: Widget, radius: Radius) -> Widget {
  let hover_layer = HoverLayer::tracked(LayerArea::WidgetCover(radius));
  ripple! {
    bounded: RippleBound::Radius(radius),
    cursor: CursorIcon::Pointer,
    @ $hover_layer { @ { w } }
  }
  .into_widget()
}
