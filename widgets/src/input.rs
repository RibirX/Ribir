mod caret;
mod caret_state;
mod handle;
mod glyphs_helper;
mod selected_text;
pub use caret::CaretStyle;
pub use caret_state::CaretState;
pub use selected_text::SelectedTextStyle;

use crate::layout::{constrained_box::EXPAND_X, ConstrainedBox, Stack, Container};
use crate::prelude::Text;

use ribir_core::{prelude::*, ticker::FrameMsg};


use self::{glyphs_helper::GlyphsHelper, selected_text::SelectedText};

#[derive(Declare)]
pub struct Input {
  #[declare(default = TypographyTheme::of(ctx).body1.text.clone())]
  pub style: TextStyle,
  #[declare(default, skip)]
  text: CowArc<str>,
  #[declare(default, skip)]
  caret: CaretState,
}

impl Input {
  pub fn text(&self) -> CowArc<str> { self.text.clone() }

  pub fn caret(&self) -> &CaretState { &self.caret }

  pub fn set_text(&mut self, text: CowArc<str>) {
    self.text = text;
    self.caret.valid(self.text.len());
  }

  pub fn set_caret(&mut self, caret: CaretState) {
    self.caret = caret;
    self.caret.valid(self.text.len());
  }
}

impl ComposeChild for Input {
  type Child = Option<Widget>;
  fn compose_child(this: StateWidget<Self>, placeholder: Self::Child) -> Widget
  where
    Self: Sized,
  {
    let end_char = "\r";
    widget! {
      states {
        this: this.into_stateful(),
        helper: GlyphsHelper::default().into_stateful(),
      }
      init ctx => {
        let tick_of_layout_ready = ctx.wnd_ctx().frame_tick_stream().filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
      }

      ConstrainedBox {
        id: outbox,
        clamp: EXPAND_X,
        auto_focus: true,
        char: move |c| this.edit_handle(c.char),
        key_down: move |key| this.key_handle(key),
        pointer_move: move |e| {
          if let CaretState::Selecting(begin, _) = this.caret {
            if e.point_type == PointerType::Mouse
              && e.mouse_buttons() == MouseButtons::PRIMARY {
              let position = to_content_pos(&container, &e.position());
              let cluster = helper.cluster_from_pos(position.x, position.y);
              this.caret = CaretState::Selecting(begin, cluster as usize);
            }
          }
        },
        pointer_down: move |e| {
          let position = to_content_pos(&container, &e.position());
          let cluster = helper.cluster_from_pos(position.x, position.y);
          this.caret = CaretState::Selecting(cluster as usize, cluster as usize);
        },
        pointer_up: move |_| {
          if let CaretState::Selecting(begin, end) = this.caret {
            this.caret = if begin == end {
             CaretState::Caret(begin as usize)
            } else {
              CaretState::Select(begin, end)
            };
          }
        },

        ScrollableWidget {
          id: container,
          scrollable: Scrollable::X,
          padding: EdgeInsets::horizontal(1.),
          Stack {
            
            SelectedText {
              id: selected,
              rects: vec![],
            }
            
            Text {
              id: text,
              text: this.text.to_string() + end_char,
              style: this.style.clone(),
    
              performed_layout: move |ctx| {
                let bound = ctx.layout_info().expect("layout info must exit in performed_layout").clamp;
                helper.glyphs = Some(Text::text_layout(
                  &text.text,
                  &text.style,
                  ctx.wnd_ctx().typography_store(),
                  bound,
                ));
              }
            }

            DynWidget {
              visible: this.text.is_empty(),
              dyns: placeholder.unwrap_or(Void {}.into_widget()),
            }

            CaretStyle{
              id: caret,
              visible: outbox.has_focus(),
              font: this.style.clone(),
              top_anchor: 0.,
              left_anchor: 0.,
              Container {
                id: icon,
                size: Size::new(1., 0.),
              }
            }
          }
        }
      }
      finally {
        let_watch!(this.caret)
          .distinct_until_changed()
          .sample(tick_of_layout_ready)
          .subscribe(move |cursor| {
            selected.rects = helper.selection(cursor.select_range());
            let (offset, height) = helper.cursor(cursor.offset());
            caret.top_anchor = PositionUnit::Pixel(offset.y);
            caret.left_anchor = PositionUnit::Pixel(offset.x);
            icon.size = Size::new(1., height);
          });
        let_watch!(caret.left_anchor.abs_value(1.))
          .scan_initial((0., 0.), |pair, v| (pair.1, v))
          .distinct_until_changed()
          .subscribe(move |(before, after)| {
            let pos = auto_x_scroll_pos(&container, before, after);
            container.silent().jump_to(Point::new(pos, 0.));
          });
      }
    }
  }
}

fn auto_x_scroll_pos(container: &ScrollableWidget, before: f32, after: f32) -> f32 {
  let view_size = container.scroll_view_size();
  let content_size = container.scroll_content_size();
  let current = container.scroll_pos.x;
  let view_after = current + after;
  let view_before = current + before;
  let inner_view = |pos| (0. <= pos && pos < view_size.width);
  if content_size.width <= view_size.width || inner_view(view_after) {
    return current;
  }
  let pos = if !inner_view(view_before) {
    view_size.width / 2.
  } else if view_after < 0. {
    0.
  } else {
    view_size.width - 1.
  };
  pos - after
}

fn to_content_pos(container: &ScrollableWidget, view_position: &Point) -> Point {
  *view_position - Size::from(container.scroll_pos.to_vector())
}
