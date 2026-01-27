use ribir_core::prelude::*;
use ribir_widgets::navigation_rail::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  rail_root_init(classes);
  rail_item_init(classes);
  rail_section_init(classes);
  rail_slots_init(classes);
}

fn rail_root_init(classes: &mut Classes) {
  classes
    .insert(NAVIGATION_RAIL, style_class! { background: Palette::of(BuildCtx::get()).surface() });

  classes.insert(NAVIGATION_RAIL_EXPANDED, style_class! { width: 256. });
  classes.insert(NAVIGATION_RAIL_COLLAPSED, style_class! { width: 80. });
  classes.insert(RAIL_CONTENT, style_class! {});
}

fn rail_item_init(classes: &mut Classes) {
  classes.insert(RAIL_ITEM, |w| {
    interactive_layers! {
      bounded: true,
      cursor: CursorIcon::Pointer,
      height: 56.,
      radius: md::RADIUS_28,
      padding: md::EDGES_HOR_16,
      margin: md::EDGES_LEFT_20,
      @ { w }
    }
    .into_widget()
  });

  classes.insert(
    RAIL_ITEM_SELECTED,
    style_class! {
      background: Palette::of(BuildCtx::get()).secondary_container(),
      foreground: Palette::of(BuildCtx::get()).on_secondary_container(),
    },
  );

  classes.insert(
    RAIL_ITEM_UNSELECTED,
    style_class! { foreground: Palette::of(BuildCtx::get()).on_surface_variant() },
  );

  classes.insert(RAIL_ITEM_ICON, style_class! { text_line_height: 24. });

  classes.insert(
    RAIL_ITEM_LABEL,
    style_class! { text_style: TypographyTheme::of(BuildCtx::get()).label_medium.text.clone() },
  );
}

fn rail_section_init(classes: &mut Classes) {
  classes.insert(RAIL_SECTION, |w| {
    fn_widget! {
      let expanded = Variant::<RailExpanded>::new_or_default(BuildCtx::get());
      @Stack {
        @Divider {
          visible: expanded.clone().map(|e| !e.0),
          indent: DividerIndent::Both
        }
        @FatObj {
          margin: EdgeInsets::new(12., 0., 8., 36.),
          visible: expanded.map(|e| e.0),
          text_style: TypographyTheme::of(BuildCtx::get()).title_small.text.clone(),
          foreground: Palette::of(BuildCtx::get()).on_surface_variant(),
          @{ w }
        }
      }
    }
    .into_widget()
  });
}

fn rail_slots_init(classes: &mut Classes) {
  classes.insert(
    RAIL_MENU,
    style_class! {
        margin: md::EDGES_HOR_36,
        text_line_height: 24.,
        cursor: CursorIcon::Pointer
    },
  );
  classes.insert(
    RAIL_ACTION,
    style_class! {
      margin: Variant::<RailExpanded>::new_or_default(BuildCtx::get()).map(|e| {
        if e.0 { EdgeInsets::new(4., 16., 12., 16.) } else { EdgeInsets::new(4., 12., 12., 12.) }
      }),
    },
  );
  classes.insert(RAIL_FOOTER, style_class! { margin: md::EDGES_16 });
}
