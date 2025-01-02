use ribir_core::prelude::*;

use crate::prelude::*;
mod caret;
mod caret_state;
mod glyphs_helper;
mod handle;

mod text_editable;
mod text_high_light;
mod text_selectable;

pub use caret::TEXT_CARET;
pub use caret_state::{CaretPosition, CaretState};
pub use handle::{EditableText, SelectableText};
pub use text_editable::{TextChanged, TextChangedEvent, edit_text};
pub use text_high_light::TEXT_HIGH_LIGHT;

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
/// fn_widget! {
///   let input_val = @Text{ text: ""};
///   @Column {
///     @Input {
///       on_custom_event: move |e: &mut TextChangedEvent| {
///         $input_val.write().text = e.data().text.clone();
///       }
///     }
///     @Row {
///       @ Text { text: "the input value is:" }
///       @ { input_val }
///     }
///   }
/// }
/// ```   

#[derive(Declare)]
pub struct Input {
  #[declare(skip)]
  text: CowArc<str>,

  #[declare(skip)]
  caret: CaretState,
}

/// The `TextArea` struct is a widget that represents a text input field
/// that displays multiple lines of text. for single line text, use `[Input]`
///
/// The TextArea will emit the [TextChanged] event when the text is changed,
/// emit the [TextSelectChanged] event when the text selection is changed.
/// The TextArea also implement the [EditableText] trait, which you can set
/// the text and the caret selection.
#[derive(Declare)]
pub struct TextArea {
  /// if true, the text will be auto wrap when the text is too long
  auto_wrap: bool,
  #[declare(skip)]
  text: CowArc<str>,
  #[declare(skip)]
  caret: CaretState,
}

impl Input {
  /// set the text and the caret selection will be reset to the start.
  pub fn set_text(&mut self, text: &str) { self.set_text_with_caret(text, CaretState::default()); }
}

impl TextArea {
  /// set the text and the caret selection will be reset to the start.
  pub fn set_text(&mut self, text: &str) { self.set_text_with_caret(text, CaretState::default()); }
}

impl SelectableText for Input {
  fn caret(&self) -> CaretState { self.caret }

  fn set_caret(&mut self, caret: CaretState) { self.caret = caret; }

  fn text(&self) -> CowArc<str> { self.text.clone() }
}

impl EditableText for Input {
  fn set_text_with_caret(&mut self, text: &str, caret: CaretState) {
    let new_text = text.replace(['\r', '\n'], " ");
    self.text = new_text.into();
    self.caret = caret;
  }
}

impl SelectableText for TextArea {
  fn text(&self) -> CowArc<str> { self.text.clone() }

  fn caret(&self) -> CaretState { self.caret }

  fn set_caret(&mut self, caret: CaretState) { self.caret = caret; }
}

impl EditableText for TextArea {
  fn set_text_with_caret(&mut self, text: &str, caret: CaretState) {
    self.text = text.to_string().into();
    self.caret = caret;
  }
}

impl Compose for Input {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @FocusScope {
        skip_host: true,
        @TextClamp {
          rows: Some(1.),
          scrollable: Scrollable::X,
          @ { edit_text(this.clone_writer()) }
        }
      }
    }
    .into_widget()
  }
}

impl Compose for TextArea {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @FocusScope {
        skip_host: true,
        @Scrollbar {
          scrollable: pipe!($this.auto_wrap).map(|v| if v {
            Scrollable::Y
          } else {
            Scrollable::Both
          }),
          text_overflow: TextOverflow::AutoWrap,
          @ { edit_text(this.clone_writer()) }
        }
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::*, reset_test_env, test_helper::*};
  use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

  use super::*;

  #[test]
  fn input_edit() {
    reset_test_env!();
    let (value, w_value) = split_value(String::default());
    let w = fn_widget! {
      let input = @Input { auto_focus: true };
      watch!($input.text.clone())
        .subscribe(move |text| *$w_value.write() = text.to_string());
      input
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    assert_eq!(*value.read(), "");

    wnd.processes_receive_chars("hello\nworld".into());
    wnd.draw_frame();
    assert_eq!(*value.read(), "hello world");
  }

  #[test]
  fn input_tap_focus() {
    reset_test_env!();
    let (value, w_value) = split_value(String::default());
    let w = fn_widget! {
      let input = @Input {  };
      watch!($input.text.clone())
        .subscribe(move |text| *$w_value.write() = text.to_string());

      @SizedBox {
        size: Size::new(200., 24.),
        @ { input }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    assert_eq!(*value.read(), "");

    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (50., 10.).into() });

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);
    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
    wnd.draw_frame();

    wnd.processes_receive_chars("hello".into());
    wnd.draw_frame();
    assert_eq!(*value.read(), "hello");
  }
}
