use super::CaretState;
use super::{glyphs_helper::GlyphsHelper, selected_text::SelectedText};
use crate::layout::{Stack, StackFit};
use crate::prelude::Text;
use ribir_core::prelude::*;

#[derive(Declare, Default)]
pub struct TextSelectable {
  #[declare(default)]
  pub caret: CaretState,
  #[declare(skip, default)]
  pub(crate) helper: GlyphsHelper,
}

impl ComposeChild for TextSelectable {
  type Child = State<Text>;
  fn compose_child(this: State<Self>, text: Self::Child) -> Widget {
    let this = this.into_writable();
    widget! {
      states {
        this: this.clone(),
        text: text.into_readonly(),
      }
      Stack {
        fit: StackFit::Passthrough,
        on_pointer_move: move |e| {
          if let CaretState::Selecting(begin, _) = this.caret {
            if e.point_type == PointerType::Mouse
              && e.mouse_buttons() == MouseButtons::PRIMARY {
              let position = e.position();
              let cluster = this.helper.cluster_from_pos(position.x, position.y);
              this.caret = CaretState::Selecting(begin, cluster as usize);
            }
          }
        },
        on_pointer_down: move |e| {
          let position = e.position();
          let cluster = this.helper.cluster_from_pos(position.x, position.y);
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

        on_key_down: move |event| {
            if event.key == VirtualKeyCode::C && event.common.with_command_key(){
                let rg = this.caret.select_range();
                if !rg.is_empty() {
                    let substr = text.text.substr(rg);
                    let clipboard = event.context().clipboard();
                    let _ = clipboard.borrow_mut().clear();
                    let _ = clipboard.borrow_mut().write_text(&substr);
                }
                event.stop_bubbling();
            }
        },
        SelectedText {
            id: selected,
            rects: vec![],
        }
        DynWidget {
          dyns: text.clone().into_widget(),
          on_performed_layout: move |ctx| {
              let bound = ctx.layout_info().expect("layout info must exit in performed_layout").clamp;
              this.helper.glyphs = Some(text.text_layout(
                ctx.wnd_ctx().typography_store(),
                bound.max,
              ));
          }
        }
      }
      finally {
        this.modifies()
          .subscribe(move |_| {
            selected.rects = this.selected_rect();
          });
      }
    }.into_widget()
  }
}

impl TextSelectable {
  pub fn cursor_layout(&self) -> (Point, f32) { self.helper.cursor(self.caret.offset()) }

  fn selected_rect(&self) -> Vec<Rect> { self.helper.selection(&self.caret.select_range()) }
}
