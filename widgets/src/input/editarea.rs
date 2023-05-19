use super::{caret::Caret, glyphs_helper::GlyphsHelper, selected_text::SelectedText};
use super::{CaretState, Placeholder};
use crate::layout::{ConstrainedBox, Stack};
use crate::prelude::Text;
use ribir_core::prelude::*;

#[derive(Declare)]
pub(crate) struct TextEditorArea {
  pub(crate) style: CowArc<TextStyle>,
  pub(crate) text: CowArc<str>,
  pub(crate) caret: CaretState,
  pub(crate) multi_line: bool,
  pub(crate) auto_wrap: bool,
}

impl ComposeChild for TextEditorArea {
  type Child = Option<Placeholder>;
  fn compose_child(this: State<Self>, placeholder: Self::Child) -> Widget {
    widget! {
      states {
        this: this.into_writable(),
        helper: Stateful::new(GlyphsHelper::default()),
      }
      init {
        let layout_ready = Subject::default();
        let mut layout_ready_emit = layout_ready.clone();
      }

      ConstrainedBox {
        id: outbox,
        clamp: BoxClamp::EXPAND_BOTH,
        on_key_down: move|key| this.key_handle(key, &helper),
        on_chars: move|ch| this.edit_handle(ch),
        on_pointer_move: move |e| {
          if let CaretState::Selecting(begin, _) = this.caret {
            if e.point_type == PointerType::Mouse
              && e.mouse_buttons() == MouseButtons::PRIMARY {
              let position = to_content_pos(&container, &e.position());
              let cluster = helper.cluster_from_pos(position.x, position.y);
              this.caret = CaretState::Selecting(begin, cluster as usize);
            }
          }
        },
        on_performed_layout: move |_| layout_ready_emit.next(()),
        on_pointer_down: move |e| {
          let position = to_content_pos(&container, &e.position());
          let cluster = helper.cluster_from_pos(position.x, position.y);
          this.caret = CaretState::Selecting(cluster as usize, cluster as usize);
        },
        on_pointer_up: move |_| {
          if let CaretState::Selecting(begin, end) = this.caret {
            this.caret = if begin == end {
             CaretState::Caret(begin)
            } else {
              CaretState::Select(begin, end)
            };
          }
        },

        ScrollableWidget {
          id: container,
          scrollable: this.scroll_dir(),
          padding: EdgeInsets::horizontal(1.),
          Stack {
            SelectedText {
              id: selected,
              rects: vec![],
            }
            Text {
              id: text,
              text: this.text.clone(),
              text_style: this.style.clone(),
              overflow: this.overflow(),
              on_performed_layout: move |ctx| {
                let bound = ctx.layout_info().expect("layout info must exit in performed_layout").clamp;
                helper.glyphs = Some(text.text_layout(
                  ctx.wnd_ctx().typography_store(),
                  bound,
                ));
              }
            }
            Option::map(placeholder, |holder| widget! {
              Text {
                visible: this.text.is_empty(),
                text: holder.0,
              }
            })

            Caret {
              id: caret,
              top_anchor: 0.,
              left_anchor: 0.,
              focused: outbox.has_focus(),
              height: 0.,
              on_performed_layout: move |ctx| {
                let size = ctx.layout_info().and_then(|info| info.size).unwrap();
                ctx.set_ime_pos(Point::new(0., size.height));
              },
            }
          }
        }
      }
      finally {
        let_watch!(this.caret)
          .distinct_until_changed()
          .sample(layout_ready)
          .subscribe(move |cursor| {
            selected.rects = helper.selection(&cursor.select_range());
            let (offset, height) = helper.cursor(cursor.offset());
            caret.top_anchor = PositionUnit::Pixel(offset.y);
            caret.left_anchor = PositionUnit::Pixel(offset.x);
            caret.height = height;
          });
        let_watch!(Point::new(caret.left_anchor.abs_value(1.), caret.top_anchor.abs_value(1.)))
          .scan_initial((Point::zero(), Point::zero()), |pair, v| (pair.1, v))
          .distinct_until_changed()
          .subscribe(move |(before, after)| {
            let pos = auto_scroll_pos(&container, before, after, caret.layout_size());
            container.silent().jump_to(pos);
          });

        // let_watch!(this.caret).distinct_until_changed() will only be triggered after modify
        // borrow mut from state_ref to manual triggered after init.
        let _:&mut TextEditorArea = &mut this;
      }
    }
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

fn to_content_pos(container: &ScrollableWidget, view_position: &Point) -> Point {
  *view_position - Size::from(container.scroll_pos.to_vector())
}
