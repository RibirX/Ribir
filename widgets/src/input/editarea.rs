use super::{caret::Caret, CaretState, Placeholder};
use crate::input::TextSelectable;
use crate::layout::{Stack, StackFit};
use crate::prelude::Text;
use ribir_core::prelude::*;
use ribir_core::ticker::FrameMsg;
use std::time::Duration;

#[derive(Declare)]
pub(crate) struct TextEditorArea {
  pub(crate) style: CowArc<TextStyle>,
  pub(crate) text: CowArc<str>,
  pub(crate) caret: CaretState,
  pub(crate) multi_line: bool,
  pub(crate) auto_wrap: bool,
}

#[derive(Clone)]
pub struct PlaceholderStyle {
  pub text_style: CowArc<TextStyle>,
  pub foreground: Brush,
}

impl CustomStyle for PlaceholderStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    Self {
      foreground: Palette::of(ctx).on_surface_variant().into(),
      text_style: TypographyTheme::of(ctx).body_medium.text.clone(),
    }
  }
}

impl ComposeChild for TextEditorArea {
  type Child = Option<State<Placeholder>>;
  fn compose_child(this: State<Self>, placeholder: Self::Child) -> Widget {
    widget! {
    states {
      this: this.into_writable(),
    }
    FocusScope {
      on_key_down: move|key| Self::key_handle(&mut this, key),
      on_chars: move|ch| Self::edit_handle(&mut this, ch),
      ScrollableWidget {
        id: container,
        scrollable: this.scroll_dir(),
        padding: EdgeInsets::horizontal(1.),
        Stack {
          fit: StackFit::Passthrough,
          Option::map(placeholder, |holder| widget! {
            states { holder: holder.into_readonly() }
            Text {
              visible: this.text.is_empty(),
              text: holder.0.clone(),
            }
          })
          TextSelectable {
            id: selectable,
            caret: this.caret,
            Text {
              text: this.text.clone(),
              text_style: this.style.clone(),
              overflow: this.overflow(),
            }
          }
          IgnorePointer{
            UnconstrainedBox {
              dir: UnconstrainedDir::Both,
              Caret {
                id: caret,
                top_anchor: 0.,
                left_anchor: 0.,
                focused: container.has_focus(),
                height: 0.,
                on_performed_layout: move |ctx| {
                  let height = ctx.box_size().unwrap().height;
                  let pos = ctx.map_to_global(Point::new(0., height));
                  ctx.window().set_ime_pos(pos);
                },
              }
            }
          }
        }
      }
    }
    finally ctx => {
      let scheduler = ctx.window().frame_scheduler();

      let_watch!(Point::new(caret.left_anchor.abs_value(1.), caret.top_anchor.abs_value(1.)))
        .scan_initial((Point::zero(), Point::zero()), |pair, v| (pair.1, v))
        .subscribe(move |(before, after)| {
          let pos = auto_scroll_pos(&container, before, after, caret.layout_size());
          container.silent().jump_to(pos);
        });

      selectable.modifies()
        .delay(Duration::ZERO, scheduler.clone())
        .subscribe(move |_| {
          if selectable.caret != this.caret {
            this.silent().caret = selectable.caret.clone();
          }
        });

      let tick_of_layout_ready = ctx.window()
        .frame_tick_stream()
        .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));

      selectable.modifies()
        .merge(observable::of(ModifyScope::BOTH))
        .sample(tick_of_layout_ready)
        .subscribe(move |_| {
          let (offset, height) = selectable.cursor_layout();
          caret.top_anchor = PositionUnit::Pixel(offset.y);
          caret.left_anchor = PositionUnit::Pixel(offset.x);
          caret.height = height;
        });
      }
    }
    .into()
  }
}

impl TextEditorArea {
  fn scroll_dir(&self) -> Scrollable {
    match (self.auto_wrap, self.multi_line) {
      (true, false) | (true, true) => Scrollable::Y,
      (false, true) => Scrollable::Both,
      (false, false) => Scrollable::X,
    }
  }

  fn overflow(&self) -> Overflow {
    match (self.auto_wrap, self.multi_line) {
      (true, false) | (true, true) => Overflow::AutoWrap,
      _ => Overflow::Clip,
    }
  }
}

fn auto_scroll_pos(container: &ScrollableWidget, before: Point, after: Point, size: Size) -> Point {
  let view_size = container.scroll_view_size();
  let content_size = container.scroll_content_size();
  let current = container.scroll_pos;
  if view_size.contains(content_size) {
    return current;
  }

  let calc_offset = |current, before, after, max_size, size| {
    let view_after = current + after;
    let view_before = current + before;
    let best_position = if !(0. <= view_before + size && view_before < max_size) {
      (max_size - size) / 2.
    } else if view_after < 0. {
      0.
    } else if view_after > max_size - size {
      max_size - size
    } else {
      view_after
    };
    current + best_position - view_after
  };
  Point::new(
    calc_offset(current.x, before.x, after.x, view_size.width, size.width),
    calc_offset(current.y, before.y, after.y, view_size.height, size.height),
  )
}
