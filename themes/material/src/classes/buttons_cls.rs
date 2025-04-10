use ribir_core::{named_style_impl, prelude::*};
use ribir_widgets::prelude::*;

use crate::*;

const BTN_40_CLAMP: BoxClamp = BoxClamp::min_width(40.).with_fixed_height(40.);

pub(super) fn init(classes: &mut Classes) {
  button_init(classes);
  filled_button_init(classes);
  text_button_init(classes);
  fab_init(classes);
}

named_style_impl!(common_btn => {
  padding: md::EDGES_HOR_16,
  text_style: btn_label_style(18.)
});
named_style_impl!(common_btn_label => { padding: md::EDGES_HOR_8 });
named_style_impl!(common_icon_only => { text_line_height: 24. });
named_style_impl!(common_label_only => {
  padding: md::EDGES_HOR_24,
  text_style: btn_label_style(40.)
});

fn text_button_init(classes: &mut Classes) {
  fn interactive(w: Widget) -> Widget {
    let mut w = base_interactive(w);
    w.foreground(BuildCtx::color())
      .clamp(BTN_40_CLAMP)
      .radius(md::RADIUS_20);
    w.into_widget()
  }

  classes.insert(
    TEXT_BTN,
    class_multi_impl![
      style_class! { padding: md::EDGES_HOR_12, text_style: btn_label_style(18.) },
      interactive
    ],
  );
  classes.insert(TEXT_BTN_LABEL, style_class! { margin: md::EDGES_HOR_4 });
  classes.insert(TEXT_BTN_LEADING_ICON, style_class! { margin: md::EDGES_RIGHT_4 });
  classes.insert(TEXT_BTN_TRAILING_ICON, style_class! { margin: md::EDGES_LEFT_4 });

  classes.insert(
    TEXT_BTN_LABEL_ONLY,
    class_multi_impl![
      style_class! { padding: md::EDGES_HOR_12, text_style: btn_label_style(40.) },
      interactive
    ],
  );
  classes.insert(TEXT_BTN_ICON_ONLY, class_multi_impl![common_icon_only, interactive]);
}

fn filled_button_init(classes: &mut Classes) {
  fn filled_interactive(w: Widget) -> Widget {
    let color = BuildCtx::color();
    let mut w = FatObj::new(w);
    w.background(color).radius(md::RADIUS_20);

    let mut w = base_interactive(w.into_widget());
    w.foreground(BuildCtx::color().on_this_color(BuildCtx::get()))
      .radius(md::RADIUS_20)
      .clamp(BTN_40_CLAMP);

    w.into_widget()
  }

  classes.insert(FILLED_BTN, class_multi_impl![common_btn, filled_interactive]);
  classes.insert(FILLED_BTN_LABEL, common_btn_label);
  classes.insert(FILLED_BTN_LEADING_ICON, empty_cls);
  classes.insert(FILLED_BTN_TRAILING_ICON, empty_cls);

  classes.insert(FILLED_BTN_LABEL_ONLY, class_multi_impl![common_label_only, filled_interactive]);
  classes.insert(FILLED_BTN_ICON_ONLY, class_multi_impl![common_icon_only, filled_interactive]);
}

fn button_init(classes: &mut Classes) {
  fn btn_interactive(w: Widget) -> Widget {
    let outline = Palette::of(BuildCtx::get()).outline();
    let mut w = FatObj::new(w);
    w.border(Border::all(BorderSide { color: outline.into(), width: 1. }));

    let mut w = base_interactive(w.into_widget());
    w.foreground(BuildCtx::color())
      .radius(md::RADIUS_20)
      .clamp(BTN_40_CLAMP);
    w.into_widget()
  }

  classes.insert(BUTTON, class_multi_impl![common_btn, btn_interactive]);
  classes.insert(BTN_LABEL, common_btn_label);
  classes.insert(BTN_LEADING_ICON, empty_cls);
  classes.insert(BTN_TRAILING_ICON, empty_cls);

  classes.insert(BTN_LABEL_ONLY, class_multi_impl![common_label_only, btn_interactive]);
  classes.insert(BTN_ICON_ONLY, class_multi_impl![common_icon_only, btn_interactive]);
}

fn fab_init(classes: &mut Classes) {
  const MINI_BTN_HEIGHT: f32 = 40.;
  const BTN_HEIGHT: f32 = 56.;
  const LARGE_BTN_HEIGHT: f32 = 96.;
  const LARGE_ICON_SIZE: f32 = 36.;

  fn fab_size() -> FabSize {
    Provider::of::<FabSize>(BuildCtx::get()).map_or(FabSize::Normal, |f| *f)
  }

  fn large_label_style(line_height: f32) -> TextStyle {
    let text_theme = TypographyTheme::of(BuildCtx::get());
    let mut text_style = text_theme.title_large.text.clone();
    text_style.line_height = line_height;
    text_style
  }

  fn fab_interactive(w: Widget) -> Widget {
    let color = BuildCtx::color();
    let ctx = BuildCtx::get();
    let background = color.clone().into_container_color(ctx);
    let foreground = color.on_this_container_color(ctx);
    let fab_size = fab_size();

    let btn_height = match fab_size {
      FabSize::Mini => MINI_BTN_HEIGHT,
      FabSize::Normal => BTN_HEIGHT,
      FabSize::Large => LARGE_BTN_HEIGHT,
    };
    let radius = match fab_size {
      FabSize::Mini => md::RADIUS_12,
      FabSize::Normal => md::RADIUS_16,
      FabSize::Large => Radius::all(28.),
    };

    let mut w = FatObj::new(w);
    w.background(background);

    let mut w = base_interactive(w.into_widget());
    w.foreground(foreground)
      .clamp(BoxClamp::min_width(btn_height).with_fixed_height(btn_height))
      .radius(radius);
    w.into_widget()
  }

  classes.insert(
    FAB_ICON_ONLY,
    class_multi_impl![
      match fab_size() {
        FabSize::Large => style_class! { text_line_height: LARGE_ICON_SIZE },
        _ => common_icon_only,
      },
      fab_interactive
    ],
  );

  classes.insert(
    FAB_LABEL_ONLY,
    class_multi_impl![
      match fab_size() {
        FabSize::Mini => common_label_only,
        FabSize::Normal =>
          style_class! { padding: md::EDGES_HOR_24, text_style: btn_label_style(BTN_HEIGHT) },
        FabSize::Large => style_class! {
          text_style: large_label_style(LARGE_BTN_HEIGHT),
          padding: md::EDGES_HOR_48,
        },
      },
      fab_interactive
    ],
  );

  classes.insert(
    FAB,
    class_multi_impl![
      match fab_size() {
        FabSize::Mini => common_btn,
        FabSize::Normal => style_class! {
          padding: md::EDGES_HOR_16,
          text_style: btn_label_style(24.)
        },
        FabSize::Large => style_class! {
          padding: md::EDGES_HOR_32,
          text_style: large_label_style(LARGE_ICON_SIZE)
        },
      },
      fab_interactive
    ],
  );
  classes.insert(FAB_LEADING_ICON, empty_cls);
  classes.insert(FAB_TRAILING_ICON, empty_cls);
  classes.insert(FAB_LABEL, |w| match fab_size() {
    FabSize::Large => {
      let mut w = FatObj::new(w);
      w.padding(md::EDGES_HOR_16);
      w.into_widget()
    }
    _ => common_btn_label(w),
  });
}

fn btn_label_style(line_height: f32) -> TextStyle {
  let text_theme = TypographyTheme::of(BuildCtx::get());
  let mut text_style = text_theme.label_large.text.clone();
  text_style.line_height = line_height;
  text_style
}

fn base_interactive(w: Widget) -> FatObj<Widget> {
  let mut w = if DisabledRipple::get(BuildCtx::get()) {
    FatObj::new(w)
  } else {
    let mut layers = InteractiveLayers::declarer();
    layers.bounded(true);
    let layers = layers.finish();
    layers.map(move |l| l.with_child(w).into_widget())
  };
  w.cursor(CursorIcon::Pointer);
  w
}
