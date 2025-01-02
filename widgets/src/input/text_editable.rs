use ribir_core::prelude::*;

use super::{
  caret::caret_widget,
  handle::{EditableText, ImeHandle},
  text_selectable::{TextSelectChanged, TextSelectChangedEvent, TextSelectable},
};
use crate::{input::text_selectable::TextSelectableDeclareExtend, prelude::*};

pub struct TextChanged {
  pub text: CowArc<str>,
  pub caret: CaretState,
}

pub type TextChangedEvent = CustomEvent<TextChanged>;

fn notify_changed(track_id: TrackId, text: CowArc<str>, caret: CaretState, wnd: &Window) {
  wnd.bubble_custom_event(track_id.get().unwrap(), TextChanged { text, caret });
}

pub fn edit_text(this: impl StateWriter<Value = impl EditableText>) -> Widget<'static> {
  fn_widget! {
    let text = @Text { text: pipe!($this.text().clone()) };
    let mut stack = @Stack {};
    let mut caret = FatObj::new(@ {
      let caret_writer = this.map_writer(|v| PartData::from_data(v.caret()));
      let text_writer = text.clone_writer();
      pipe!($stack.has_focus()).map(move |v|
        if v {
          caret_widget(caret_writer.clone_watcher(), text_writer.clone_watcher())
        } else {
          @Void{}.into_widget()
        }
      )
    });

    let wnd = BuildCtx::get().window();
    let ime = Stateful::new(ImeHandle::new(this.clone_writer()));
    watch!($caret.layout_rect())
      .scan_initial((Rect::zero(), Rect::zero()), |pair, v| (pair.1, v))
      .subscribe(move |(mut before, mut after)| {
        if let Some(wid) = $stack.track_id().get() {
          let offset = wnd.map_to_global(Point::zero(), wid).to_vector();
          after.origin += offset;
          before.origin += offset;
          wnd.bubble_custom_event(wid, ScrollRequest::new(move |view_info: ScrollViewInfo| {
            auto_scroll_pos(view_info.current, view_info.global_view, before, after)
          }));
          if $stack.has_focus() && !$ime.is_in_pre_edit(){
            wnd.set_ime_cursor_area(&after);
          }
        }
    });

    @ $stack {
      on_focus: move |e| { e.window().set_ime_allowed(true); },
      on_blur: move |e| { e.window().set_ime_allowed(false); },
      on_chars: move |c| {
        let mut this = $this.write();
        if this.chars_handle(c) {
          notify_changed($stack.track_id(), this.text().clone(), this.caret(), &c.window());
        } else {
          this.forget_modifies();
        }
      },
      on_key_down: move |k| {
        let mut this = $this.write();
        if this.keys_handle(&$text, k) {
          notify_changed($stack.track_id(), this.text().clone(), this.caret(), &k.window());
        } else {
          this.forget_modifies();
        }
      },
      on_ime_pre_edit: move|e| {
        $ime.write().process_pre_edit(e);
        notify_changed($stack.track_id(), $this.text().clone(), $this.caret(), &e.window());
      },
      @ TextSelectable {
        caret: pipe!($this.caret()),
        margin: pipe!($caret.layout_size()).map(|v|EdgeInsets::only_right(v.width)),
        on_custom_event: move |e: &mut TextSelectChangedEvent| {
          let TextSelectChanged { text, caret } = e.data();
          if text == &$this.text() {
            $this.write().set_caret(*caret);
          }
        },
        @ { text }
      }
      @ { caret }
    }
  }
  .into_widget()
}

fn auto_scroll_pos(scroll_pos: Point, view_rect: Rect, mut before: Rect, mut after: Rect) -> Point {
  if view_rect.contains_rect(&after) {
    return scroll_pos;
  }
  before = before.translate(-view_rect.origin.to_vector());
  after = after.translate(-view_rect.origin.to_vector());

  let calc_offset = |before_min, before_max, after_min, after_max, view_size| {
    let size = after_max - after_min;
    let best_position = if before_min < 0. || view_size < before_max {
      (view_size - size) / 2.
    } else if after_min < 0. {
      0.
    } else if view_size < after_max {
      view_size - size
    } else {
      after_min
    };

    after_min - best_position
  };

  let offset = Point::new(
    calc_offset(before.min_x(), before.max_x(), after.min_x(), after.max_x(), view_rect.width()),
    calc_offset(before.min_y(), before.max_y(), after.min_y(), after.max_y(), view_rect.height()),
  );
  scroll_pos + offset.to_vector()
}
