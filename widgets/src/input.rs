use std::ops::Range;

use ribir_core::prelude::*;

use crate::prelude::*;

mod edit_text;
mod text_glyphs;

mod text_editable;
mod text_selectable;

pub use edit_text::*;
pub use text_editable::*;
pub use text_glyphs::*;
pub use text_selectable::*;

class_names!(
  ///Class name for the input widget
  INPUT,
  ///Class name for the text area widget
  TEXTAREA,
);

/// The `Input` struct is a widget that represents a text input field
/// that displays a single line of text. if you need multi line text, use
/// `[TextArea]`
///
/// The Input will emit the [TextChangedEvent] event when the text is changed,
/// emit the [TextSelectChanged] event when the text selection is changed.
/// The Input also implement the [EditableText] trait, which you can set
/// the text and the caret selection.
///
/// ## Example
///
/// ```rust no_run
/// use ribir::prelude::*;
/// let w = fn_widget! {
///   let input = @Input {};
///   @Column {
///     @ Text { text: pipe!("the input value is:".to_string() + $read(input).text()) }
///     @ Row {
///       @ Text { text: "input value:" }
///       @ { input }
///     }
///   }
/// };
/// App::run(w);
/// ```
#[declare]
pub struct Input {
  #[declare(skip)]
  basic: BasicEditor<InputText>,
}

impl Input {
  /// set the text and the caret selection will be reset to the start.
  pub fn set_text(&mut self, text: &str) {
    let v = text
      .chars()
      .filter(|c| *c != '\n' && *c != '\r')
      .collect::<String>();
    *self.basic.text_mut() = InputText::new(v);
    let selection = &mut self.basic.selection;
    selection.from = CaretPosition::default();
    selection.to = CaretPosition::default();
  }

  pub fn text(&self) -> &CowArc<str> { &self.basic.text().0 }

  /// set the caret selection, and the caret position will be set to the `to`
  /// cluster
  pub fn select(&mut self, from: usize, to: usize) {
    let selection = &mut self.basic.selection;
    selection.from = CaretPosition { cluster: from, position: None };
    selection.to = CaretPosition { cluster: to, position: None };
  }

  /// return the selection range of the text
  pub fn selection(&self) -> Range<usize> { self.basic.cluster_rg() }
}

/// The `TextArea` struct is a widget that represents a text input field
/// that displays multiple lines of text. for single line text, use `[Input]`
#[declare]
pub struct TextArea {
  /// if true, the text will be auto wrap when the text is too long
  #[declare(default = true)]
  auto_wrap: bool,
  #[declare(skip)]
  basic: BasicEditor<CowArc<str>>,
}

impl TextArea {
  /// set the text and the caret selection will be reset to the start.
  pub fn set_text(&mut self, text: &str) {
    *self.basic.text_mut() = text.to_string().into();
    let selection = &mut self.basic.selection;
    selection.from = CaretPosition::default();
    selection.to = CaretPosition::default();
  }

  pub fn text(&self) -> &CowArc<str> { self.basic.text() }

  /// set the caret selection, and the caret position will be set to the `to`
  /// cluster
  pub fn select(&mut self, from: usize, to: usize) {
    let selection = &mut self.basic.selection;
    selection.from = CaretPosition { cluster: from, position: None };
    selection.to = CaretPosition { cluster: to, position: None };
  }

  /// return the selection range of the text
  pub fn selection(&self) -> Range<usize> { self.basic.cluster_rg() }
}

#[derive(Clone, Eq, PartialEq, Default)]
pub struct InputText(CowArc<str>);
impl InputText {
  pub fn new(v: impl Into<CowArc<str>>) -> Self { InputText(v.into()) }
  pub fn text(&self) -> &CowArc<str> { &self.0 }
}

impl BaseText for InputText {
  fn len(&self) -> usize { self.0.len() }
  fn substr(&self, rg: Range<usize>) -> Substr { self.0.substr(rg) }
  fn measure_bytes(&self, byte_from: usize, char_len: isize) -> usize {
    self.0.measure_bytes(byte_from, char_len)
  }
  fn select_token(&self, byte_from: usize) -> Range<usize> { self.0.select_token(byte_from) }
}

impl VisualText for InputText {
  fn layout_glyphs(&self, clamp: BoxClamp, ctx: &LayoutCtx) -> VisualGlyphs {
    self.0.layout_glyphs(clamp, ctx)
  }

  fn paint(&self, painter: &mut Painter, style: PaintingStyle, glyphs: &VisualGlyphs, rect: Rect) {
    self.0.paint(painter, style, glyphs, rect);
  }
}

impl EditText for InputText {
  fn insert_str(&mut self, at: usize, v: &str) -> usize {
    let new_v = v
      .chars()
      .filter(|c| *c != '\n' && *c != '\r')
      .collect::<String>();
    self.0.insert_str(at, new_v.as_str())
  }

  fn del_rg_str(&mut self, rg: Range<usize>) -> Range<usize> { self.0.del_rg_str(rg) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CaretPosition {
  /// the cluster of the caret
  pub cluster: usize,
  /// the position of the caret, it may be set by the ui interaction
  pub position: Option<(usize, usize)>,
}

impl Compose for Input {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    focus_scope! {
      skip_host: true,
      @TextClamp {
        rows: Some(1.),
        cols: Some(20.),
        class: INPUT,
        @FatObj {
          scrollable: Scrollable::X,
          @part_writer!(&mut this.basic)
        }
      }
    }
    .into_widget()
  }
}

impl Compose for TextArea {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    focus_scope! {
      @TextClamp {
        rows: Some(2.),
        cols: Some(20.),
        class: TEXTAREA,
        @Scrollbar {
          text_overflow: TextOverflow::AutoWrap,
          @part_writer!(&mut this.basic)
        }
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::*, reset_test_env, test_helper::*};

  use super::*;

  #[test]
  fn input_edit() {
    reset_test_env!();
    let (value, w_value) = split_value(String::default());
    let w = fn_widget! {
      let input = @Input { auto_focus: true };
      watch!($read(input).text().clone())
        .subscribe(move |text| *$write(w_value) = text.to_string());
      input
    };

    let wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    assert_eq!(*value.read(), "");

    wnd.process_receive_chars("hello\nworld".into());
    wnd.draw_frame();
    assert_eq!(*value.read(), "helloworld");
  }

  #[test]
  fn input_tap_focus() {
    reset_test_env!();
    let (value, w_value) = split_value(String::default());
    let w = fn_widget! {
      let input = @Input {  };
      watch!($read(input).text().clone())
        .subscribe(move |text| *$write(w_value) = text.to_string());

      @SizedBox {
        size: Size::new(200., 24.),
        @ { input }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    assert_eq!(*value.read(), "");

    wnd.process_cursor_move(Point::new(50., 10.));

    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();

    wnd.process_receive_chars("hello".into());
    wnd.draw_frame();
    assert_eq!(*value.read(), "hello");
  }
}
