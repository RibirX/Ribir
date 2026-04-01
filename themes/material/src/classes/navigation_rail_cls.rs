use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::{
  DisableRippleLayer, InteractiveLayers, md,
  md::nav_rail::{collapsed, common, expanded},
};

pub(super) fn init(classes: &mut Classes) {
  rail_root_init(classes);
  rail_header_init(classes);
  rail_content_init(classes);
  rail_item_init(classes);
  rail_section_init(classes);
  rail_slots_init(classes);
}

fn smooth_pos<'c>(child: Widget<'c>) -> Widget<'c> {
  smooth_layout! {
    transition: md::motion::spring::spatial::default(),
    pos_axes: PosAxes::Pos,
    size_axes: SizeAxes::None,
    @ { child }
  }
  .into_widget()
}

fn smooth_y<'c>(child: Widget<'c>) -> Widget<'c> {
  smooth_layout! {
    transition: md::motion::spring::spatial::default(),
    pos_axes: PosAxes::Y,
    size_axes: SizeAxes::None,
    @ { child }
  }
  .into_widget()
}

fn rail_root_init(classes: &mut Classes) {
  classes.insert(NAVIGATION_RAIL, |w| {
    smooth_layout! {
      transition: md::motion::spring::spatial::default(),
      size_mode: SizeMode::Visual,
      @FatObj {
        height: Measure::Unit(1.),
        padding: common::CONTAINER_PADDING,
        background: Palette::of(BuildCtx::get()).surface(),
        @ { w }
      }
    }
    .into_widget()
  });

  classes.insert(
    NAVIGATION_RAIL_EXPANDED,
    style_class! { min_width: expanded::MIN_WIDTH, max_width: expanded::MAX_WIDTH},
  );
  classes.insert(NAVIGATION_RAIL_COLLAPSED, style_class! { width: collapsed::WIDTH });
}

fn rail_header_init(classes: &mut Classes) {
  classes.insert(RAIL_HEADER, style_class! { margin: common::HEADER_MARGIN });
}

fn rail_content_init(classes: &mut Classes) {
  classes
    .insert(RAIL_CONTENT_NO_HEADER, style_class! { margin: common::CONTENT_TOP_MARGIN_NO_HEADER });
}

fn rail_item_init(classes: &mut Classes) {
  fn indicator() -> Widget<'static> {
    fn_widget! {
      let ctx = BuildCtx::get();
      let key = RailItem::of(ctx)
        .map(|item| item.key.clone())
        .expect("RAIL_ITEM should only be used in RailItem");
      let selected = Provider::state_of::<Stateful<NavigationRail>>(ctx)
        .expect("RAIL_ITEM_ICON should only be used in NavigationRail")
        .part_watcher(|nav| PartRef::from(nav.selected()));

      let is_selected = distinct_pipe!(*$read(selected) == key);

      let mut inner = @Void {
        background: Palette::of(BuildCtx::get()).secondary_container(),
        x: AnchorX::center(),
        hint_size: expanded_switch(
          expanded::INDICATOR_SIZE,
          collapsed::INDICATOR_SIZE.into(),
        ),
        radius: expanded_switch(expanded::item::RADIUS, collapsed::INDICATOR_RADIUS),
      };
      let fade_trans = md::motion::spring::spatial::default();
      inner.opacity().transition_with_init(0., fade_trans);
      inner.with_opacity(is_selected.map(|v| if v { 1. } else { 0. }));

      @SmoothLayout {
        transition: md::motion::spring::spatial::default(),
        pos_axes: PosAxes::None,
        size_axes: SizeAxes::Size,
        size_mode: SizeMode::Visual,
        size_effect: SizeEffect::Scale,
        @ { inner }
      }
    }
    .into_widget()
  }

  classes.insert(RAIL_ITEM, |w| {
    let item = stack! {
      margin: expanded_switch(expanded::item::MARGIN, collapsed::item::MARGIN),
      @InParentLayout { @indicator() }
      @FatObj {
        height: common::item::HEIGHT,
        clamp: expanded_switch(
          BoxClamp::min_width(common::item::MIN_WIDTH),
          BoxClamp::fixed_width(collapsed::item::SLOT_WIDTH)
        ),
        padding: expanded_switch(expanded::item::PADDING, collapsed::item::PADDING),
        @ { w }
      }
      @InParentLayout {
        @InteractiveLayers {
          providers: [Provider::new(DisableRippleLayer)],
          bounded: true,
          cursor: CursorIcon::Pointer,
          x: AnchorX::center(),
          radius: expanded_switch(expanded::item::RADIUS, collapsed::INDICATOR_RADIUS),
          @Void {
            hint_size: expanded_switch(
              expanded::INDICATOR_SIZE,
              collapsed::INDICATOR_SIZE.into(),
            ),
          }
        }
      }
    };
    item.into_widget()
  });

  classes.insert(RAIL_ITEM_SELECTED, |w| {
    let effects_trans = md::motion::spring::effects::fast();
    let palette = Palette::of(BuildCtx::get());

    let mut item = FatObj::new(w);
    item
      .foreground()
      .transition_with_init(palette.on_surface_variant().into(), effects_trans);
    item.with_foreground(palette.secondary());
    item.into_widget()
  });

  classes.insert(RAIL_ITEM_UNSELECTED, |w| {
    let effects_trans = md::motion::spring::effects::fast();
    let palette = Palette::of(BuildCtx::get());

    let mut item = FatObj::new(w);
    item
      .foreground()
      .transition_with_init(palette.secondary().into(), effects_trans);
    item.with_foreground(palette.on_surface_variant());
    item.into_widget()
  });

  classes.insert(RAIL_ITEM_ICON, |w| {
    smooth_y(
      fn_widget! {
        @FatObj {
          text_line_height: md::ICON_SIZE,
          @ { w }
        }
      }
      .into_widget(),
    )
  });

  classes.insert(RAIL_ITEM_LABEL, |w| {
    fn_widget! {
      let mut label = @FatObj {
        margin: expanded_switch(
          expanded::item::LABEL_MARGIN,
          collapsed::item::LABEL_MARGIN
        ),
        text_style: expanded_switch(
          expanded::item::text_style(),
          collapsed::item::text_style()
        ),
        @ { w }
      };
      // M3 Expressive: label transitions use spatial fast (350ms, damping 0.9)
      // for a snappy take-off with soft landing — not effects slow (300ms, damping 1.0)
      // which feels abrupt and critically-damped.
      let label_trans = md::motion::spring::spatial::default();
      let fade_in_out = @Animate {
        transition: label_trans,
        state: label.opacity(),
        from: 0.,
      };

      let ctx = BuildCtx::get();

      if let Some(Variant::Watcher(expanded)) = Variant::<RailExpanded>::new(ctx) {
        let u = watch!($read(expanded).0)
          .distinct_until_changed()
          .subscribe(move |_| {
            let animate = $writer(fade_in_out);
            animate.write().from = 0.;
            animate.run();
          });
        label.on_disposed(move |_| u.unsubscribe());
      }
      if let Some(item) = RailItem::of(ctx) {
        let visible = item.label_visible();
        if matches!(visible, PipeValue::Pipe{ ..}) {
          // Configure transition before binding opacity value, so the initial
          // visible-state sync can animate from init opacity.
          label.opacity().transition_with_init(0., label_trans);
          label.with_opacity(visible.map(|v| if v { 1. } else { 0. }));
        }
      }

      label
    }
    .into_widget()
  });
}

fn rail_section_init(classes: &mut Classes) {
  classes.insert(RAIL_SECTION, |w| {
    smooth_pos(
      fn_widget! {
        @FatObj {
          // The section not visible when rail is collapsed
          margin: expanded::section::MARGIN,
          text_style: expanded::section::text_style(),
          foreground: Palette::of(BuildCtx::get()).on_surface_variant(),
          @ { w }
        }
      }
      .into_widget(),
    )
  });
}

fn rail_slots_init(classes: &mut Classes) {
  classes.insert(RAIL_MENU, |w| {
    fn_widget! {
      @FatObj {
        margin: expanded_switch(expanded::MENU_MARGIN, collapsed::MENU_MARGIN),
        size: md::ICON_BTN_SIZE,
        text_line_height: md::ICON_SIZE,
        cursor: CursorIcon::Pointer,
        @ { w }
      }
    }
    .into_widget()
  });
  classes.insert(RAIL_ACTION, |w| {
    fn_widget! {
      @FatObj {
        margin: expanded_switch(expanded::ACTION_MARGIN, collapsed::ACTION_MARGIN),
        @ { w }
      }
    }
    .into_widget()
  });
  classes.insert(RAIL_ACTION_WITH_MENU, |w| {
    fn_widget! {
      @FatObj {
        margin: common::ACTION_WITH_MENU_MARGIN,
        @ { w }
      }
    }
    .into_widget()
  });
  classes.insert(
    RAIL_FOOTER,
    style_class! {
      margin: expanded_switch(expanded::FOOTER_MARGIN, collapsed::FOOTER_MARGIN)
    },
  );
}
