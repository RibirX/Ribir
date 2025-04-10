use ribir_core::prelude::*;
use ribir_widgets::list::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    LIST,
    style_class! {
      // Default line height for the icon.
      text_line_height: 24.,
      margin: md::EDGES_VER_8,
    },
  );

  classes.insert(
    LIST_ITEM_SELECTED,
    style_class! {
      background: Palette::of(BuildCtx::get()).secondary_container(),
      foreground: Palette::of(BuildCtx::get()).on_secondary_container(),
    },
  );

  classes.insert(
    LIST_ITEM_UNSELECTED,
    style_class! {
      background: Palette::of(BuildCtx::get()).surface(),
      foreground: Palette::of(BuildCtx::get()).on_surface(),
    },
  );

  classes.insert(LIST_ITEM_INTERACTIVE, |w| {
    if DisabledRipple::get(BuildCtx::get()) {
      return w;
    }

    interactive_layers! {
      cursor: CursorIcon::Pointer,
      bounded: true,
      @{ w }
    }
    .into_widget()
  });
  classes.insert(LIST_ITEM, |w| {
    let mut w = FatObj::new(w);
    let margin =
      pipe!($w.layout_height()).map(|h| if h >= 64. { md::EDGES_VER_12 } else { md::EDGES_VER_8 });

    // The `List` widget uses the `ListItemAlignItems` provider to control the
    // alignment of its child items.
    let (align_provider, u) = Stateful::from_pipe(pipe! {
      let align = if $w.layout_height() >= 64. { Align::Start } else { Align::Center };
      ListItemAlignItems(align)
    });

    w.providers(smallvec::smallvec![Provider::value_of_watcher(align_provider),])
      .clamp(BoxClamp::min_height(40.))
      .margin(margin)
      .on_disposed(|_| u.unsubscribe());
    w.into_widget()
  });
  classes.insert(
    LIST_ITEM_CONTENT,
    style_class! {
      providers: [Provider::new(TextAlign::Start)],
      margin: md::EDGES_HOR_16,
    },
  );
  classes.insert(
    LIST_ITEM_HEADLINE,
    style_class! {
      text_style: TypographyTheme::of(BuildCtx::get()).body_large.text.clone(),
    },
  );

  classes.insert(
    LIST_ITEM_SUPPORTING,
    style_class! {
      clip_boundary: true,
      text_style: {
        let style = TypographyTheme::of(BuildCtx::get()).body_medium.text.clone();
        style.with_overflow(TextOverflow::AutoWrap)
      }
    },
  );

  classes.insert(
    LIST_ITEM_TRAILING_SUPPORTING,
    style_class! {
      margin: md::EDGES_RIGHT_16,
      text_style: TypographyTheme::of(BuildCtx::get()).label_small.text.clone(),
    },
  );

  classes.insert(LIST_ITEM_LEADING, style_class! { margin: md::EDGES_LEFT_16 });

  /// Ensures proper spacing for the trailing widget in a list item.
  /// If the item does not support content, there may be excessive space between
  /// the headline and trailing widget. This function ensures the trailing
  /// widget is at least 48px wide to avoid it being too close to the right
  /// edge.
  fn ensure_trailing_spacing(widget: Widget) -> Widget {
    let struct_info = Provider::of::<ListItemStructInfo>(BuildCtx::get());
    let needs_spacing =
      struct_info.is_some_and(|info| !info.supporting && !info.trailing_supporting);
    let mut widget = FatObj::new(widget);

    if needs_spacing {
      container! {
        size: md::SIZE_48,
        @ $widget {
            h_align: HAlign::Center,
            v_align: VAlign::Center,
        }
      }
      .into_widget()
    } else {
      widget.clamp(BoxClamp::max_size(md::SIZE_48));
      widget.into_widget()
    }
  }

  classes.insert(
    LIST_ITEM_TRAILING,
    class_multi_impl![ensure_trailing_spacing, style_class! { margin: md::EDGES_RIGHT_16 }],
  );

  classes.insert(
    LIST_ITEM_IMG,
    style_class! {
      clamp: BoxClamp::fixed_size(Size::splat(56.)),
      box_fit: BoxFit::Contain
    },
  );

  classes.insert(
    LIST_ITEM_THUMB_NAIL,
    style_class! {
      // Align thumbnail to the left edge by applying negative margin
      margin: EdgeInsets::only_left(-16.),
      clamp: BoxClamp::fixed_height(64.),
      box_fit: BoxFit::Contain,
    },
  );
}
