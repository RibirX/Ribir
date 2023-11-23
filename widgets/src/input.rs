use ribir_core::{prelude::*, ticker::FrameMsg};
mod caret;
mod caret_state;
mod glyphs_helper;
mod handle;
mod selected_text;
mod text_selectable;
use crate::{
  input::{
    caret::Caret,
    handle::{edit_handle, edit_key_handle, TextCaretWriter},
    selected_text::SelectedHighLight,
    text_selectable::{bind_point_listener, select_key_handle, SelectableText},
  },
  layout::{ConstrainedBox, OnlySizedByParent, Stack, StackFit},
  prelude::Text,
};
pub use caret_state::{CaretPosition, CaretState};
pub use selected_text::SelectedHighLightStyle;
use std::ops::Range;
pub use text_selectable::TextSelectable;

pub struct Placeholder(CowArc<str>);

impl Placeholder {
  #[inline]
  pub fn new(str: impl Into<CowArc<str>>) -> Self { Self(str.into()) }
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

#[derive(Clone, PartialEq)]
pub struct InputStyle {
  pub size: Option<f32>,
}

impl CustomStyle for InputStyle {
  fn default_style(_: &BuildCtx) -> Self { InputStyle { size: Some(20.) } }
}

#[derive(Clone, PartialEq)]
pub struct TextAreaStyle {
  pub rows: Option<f32>,
  pub cols: Option<f32>,
}

impl CustomStyle for TextAreaStyle {
  fn default_style(_: &BuildCtx) -> Self { TextAreaStyle { rows: Some(2.), cols: Some(20.) } }
}

pub trait EditableText: Sized {
  fn text(&self) -> &CowArc<str>;

  fn caret(&self) -> CaretState;

  fn set_text_with_caret(&mut self, text: &str, caret: CaretState);

  fn writer(&mut self) -> TextCaretWriter<Self> { TextCaretWriter::new(self) }
}

#[derive(Declare)]
pub struct Input {
  #[declare(default = TypographyTheme::of(ctx!()).body_large.text.clone())]
  pub style: CowArc<TextStyle>,
  #[declare(skip)]
  text: CowArc<str>,
  #[declare(skip)]
  caret: CaretState,
  #[declare(default = InputStyle::of(ctx!()).size)]
  size: Option<f32>,
}

#[derive(Declare)]
pub struct TextArea {
  #[declare(default = TypographyTheme::of(ctx!()).body_large.text.clone())]
  pub style: CowArc<TextStyle>,
  #[declare(default = true)]
  pub auto_wrap: bool,
  #[declare(skip)]
  text: CowArc<str>,
  #[declare(skip)]
  caret: CaretState,
  #[declare(default = TextAreaStyle::of(ctx!()).rows)]
  rows: Option<f32>,
  #[declare(default = TextAreaStyle::of(ctx!()).cols)]
  cols: Option<f32>,
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
  fn select_range(&self) -> Range<usize> { self.caret.select_range() }

  fn text(&self) -> &CowArc<str> { &self.text }

  fn caret(&self) -> CaretState { self.caret }

  fn set_caret(&mut self, caret: CaretState) { self.caret = caret; }
}

impl EditableText for Input {
  fn text(&self) -> &CowArc<str> { &self.text }
  fn caret(&self) -> CaretState { self.caret }
  fn set_text_with_caret(&mut self, text: &str, caret: CaretState) {
    let new_text = text.replace(['\r', '\n'], " ");
    self.text = new_text.into();
    self.caret = caret;
  }
}

impl SelectableText for TextArea {
  fn select_range(&self) -> Range<usize> { self.caret.select_range() }

  fn text(&self) -> &CowArc<str> { &self.text }

  fn caret(&self) -> CaretState { self.caret }

  fn set_caret(&mut self, caret: CaretState) { self.caret = caret; }
}

impl EditableText for TextArea {
  fn text(&self) -> &CowArc<str> { &self.text }

  fn caret(&self) -> CaretState { self.caret }

  fn set_text_with_caret(&mut self, text: &str, caret: CaretState) {
    self.text = text.to_string().into();
    self.caret = caret;
  }
}

#[derive(Debug)]
struct PreEditState {
  position: usize,
  value: Option<String>,
}

#[derive(Default)]
struct PreEditHandle(Option<PreEditState>);

impl PreEditHandle {
  fn update<H>(
    &mut self,
    host: &impl StateWriter<Value = H>,
    e: &ImePreEditEvent,
    caret_position: &Point,
    caret_size: &Size,
  ) where
    H: EditableText + 'static,
  {
    match &e.pre_edit {
      ImePreEdit::Begin => {
        let mut host = host.write();
        let rg = host.caret().select_range();
        host.writer().delete_byte_range(&rg);

        self.0 = Some(PreEditState { position: rg.start, value: None });
        let pos = e.map_to_global(*caret_position);
        let height = caret_size.height;
        e.window()
          .set_ime_cursor_area(&Rect::new(Point::new(pos.x, pos.y + height), *caret_size));
      }
      ImePreEdit::PreEdit { value, cursor } => {
        let Some(PreEditState { position, value: edit_value }) = self.0.as_mut() else {
          return;
        };
        let mut host = host.write();
        let mut writer = host.writer();
        if let Some(txt) = edit_value {
          writer.delete_byte_range(&(*position..*position + txt.len()));
        }
        writer.insert_str(value);
        writer.set_to(*position + cursor.map_or(0, |(start, _)| start));
        *edit_value = Some(value.clone());
      }
      ImePreEdit::End => {
        if let Some(PreEditState { value: Some(txt), position }) = self.0.take() {
          let mut host = host.write();
          let mut writer = host.writer();
          writer.delete_byte_range(&(position..position + txt.len()));
        }
      }
    }
  }
}

impl ComposeChild for Input {
  type Child = Option<State<Placeholder>>;
  fn compose_child(
    this: impl StateWriter<Value = Self>,
    placeholder: Self::Child,
  ) -> impl WidgetBuilder {
    fn_widget! {
      let text = @Text {
        text: pipe!($this.text.clone()),
        text_style: pipe!($this.style.clone()),
      }.into_inner();
      @FocusScope {
        @ConstrainedBox {
          clamp: pipe!(size_clamp(&$this.style, Some(1.), $this.size)),
          @ {
            EditableTextExtraWidget::edit_area(
              &this,
              text,
              BoxPipe::value(Scrollable::X).into_pipe(),
              placeholder
            )
          }
        }
      }
    }
  }
}

impl ComposeChild for TextArea {
  type Child = Option<State<Placeholder>>;
  fn compose_child(
    this: impl StateWriter<Value = Self>,
    placeholder: Self::Child,
  ) -> impl WidgetBuilder {
    fn_widget! {
      let text = @Text {
        text: pipe!($this.text.clone()),
        text_style: pipe!($this.style.clone()),
        overflow: pipe!($this.auto_wrap).map(|auto_wrap| match auto_wrap {
          true => Overflow::AutoWrap,
          false => Overflow::Clip,
        }),
      }.into_inner();

      let scroll_dir = pipe!($this.auto_wrap).map(|auto_wrap| match auto_wrap {
        true => Scrollable::Y,
        false => Scrollable::Both,
      });
      @FocusScope {
        @ConstrainedBox {
          clamp: pipe!(size_clamp(&$this.style, $this.rows, $this.cols)),
          @ { EditableTextExtraWidget::edit_area(&this, text, scroll_dir, placeholder) }
        }
      }
    }
  }
}

trait EditableTextExtraWidget: EditableText + SelectableText
where
  Self: 'static,
{
  fn edit_area(
    this: &impl StateWriter<Value = Self>,
    text: State<Text>,
    scroll_dir: impl Pipe<Value = Scrollable> + 'static,
    placeholder: Option<State<Placeholder>>,
  ) -> impl WidgetBuilder {
    fn_widget! {
      let mut text = @$text{};
      let layout_box = text.get_builtin_layout_box(ctx!()).clone_reader();
      let only_text = text.clone_reader();

      let mut stack = @Stack {
        fit: StackFit::Passthrough,
        scrollable: scroll_dir,
      };

      let mut caret_box = @ConstrainedBox {
        left_anchor: pipe!($this.caret_position(&$text, $text.layout_size()).map_or(0., |p| p.x)),
        top_anchor: pipe!($this.caret_position(&$text, $text.layout_size()).map_or(0., |p| p.y)),
        clamp: pipe!(
            $this.current_line_height(&$text, $text.layout_size()).unwrap_or(0.)
          ).map(BoxClamp::fixed_height),
      };

      let scrollable = stack.get_builtin_scrollable_widget(ctx!());
      let tick_of_layout_ready = ctx!().window()
        .frame_tick_stream()
        .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
      watch!(Point::new($caret_box.left_anchor, $caret_box.top_anchor))
        .distinct_until_changed()
        .sample(tick_of_layout_ready)
        .scan_initial((Point::zero(), Point::zero()), |pair, v| (pair.1, v))
        .subscribe(move |(before, after)| {
          let mut scrollable = $scrollable.silent();
          let pos = auto_scroll_pos(&scrollable, before, after, $caret_box.layout_size());
          scrollable.jump_to(pos);
      });

      let placeholder = @ {
        placeholder.map(move |holder| @Text {
          visible: pipe!(SelectableText::text(&*$this).is_empty()),
          text: pipe!((*$holder).0.clone()),
        })
      };

      let mut pre_edit_handle = PreEditHandle::default();
      let mut stack = @ $stack {
        on_focus: move |e| e.window().set_ime_allowed(true),
        on_blur: move |e| e.window().set_ime_allowed(false),
        on_chars: move |c| {
          let _hint_capture_writer = || $this.write();
          edit_handle(&this, c);
        },
        on_key_down: move |k| {
          let _hint_capture_writer = || $this.write();
          select_key_handle(&this, &$only_text, &$layout_box, k);
          edit_key_handle(&this, k);
        },
        on_ime_pre_edit: move |e| {
          let _hint_capture_writer = || $this.write();
          let base = $stack.scroll_pos;
          let caret_position =
            Point::new($caret_box.left_anchor, $caret_box.top_anchor) + base.to_vector();
          let caret_size = $caret_box.layout_size();
          pre_edit_handle.update(&this, e, &caret_position, &caret_size);
        },
      };

      let high_light_rect = @UnconstrainedBox {
        clamp_dim: ClampDim::MIN_SIZE,
        @OnlySizedByParent {
          @SelectedHighLight {
            visible: pipe!($stack.has_focus()),
            rects: pipe! {
              $this.select_text_rect(&$only_text, $text.layout_size())
            }
          }
        }
      };

      let caret = @UnconstrainedBox {
        clamp_dim: ClampDim::MIN_SIZE,
        @OnlySizedByParent {
          @$caret_box {
            @Caret { focused: pipe!($stack.has_focus()) }
          }
        }
      };

      let text_widget = text.widget_build(ctx!());
      let text_widget = bind_point_listener(
        this.clone_writer(),
        text_widget,
        only_text,
        layout_box
      );

      @ $stack {
        padding: EdgeInsets::horizontal(2.),
        @ { placeholder }
        @ { high_light_rect }
        @ { caret }
        @ { text_widget }
      }
    }
  }
}

impl EditableTextExtraWidget for TextArea {}

impl EditableTextExtraWidget for Input {}

fn size_clamp(style: &TextStyle, rows: Option<f32>, cols: Option<f32>) -> BoxClamp {
  let mut clamp: BoxClamp = BoxClamp {
    min: Size::new(0., 0.),
    max: Size::new(f32::INFINITY, f32::INFINITY),
  };
  if let Some(cols) = cols {
    let width = cols * style.font_size.into_pixel().value();
    clamp = clamp.with_fixed_width(width);
  }
  if let Some(rows) = rows {
    let height: Pixel = style
      .line_height
      .unwrap_or(style.font_size.into_em())
      .into();

    clamp = clamp.with_fixed_height(rows * height.value());
  }
  clamp
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

#[cfg(test)]
mod tests {
  use super::{EditableText, Input};
  use crate::layout::SizedBox;
  use ribir_core::{prelude::*, reset_test_env, test_helper::TestWindow};
  use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

  fn split_value<T: 'static>(v: T) -> (impl StateReader<Value = T>, impl StateWriter<Value = T>) {
    let src = Stateful::new(v);
    (src.clone_reader(), src.clone_writer())
  }

  #[test]
  fn input_edit() {
    reset_test_env!();
    let (input_value, input_value_writer) = split_value(String::default());
    let w = fn_widget! {
      let input = @Input {
        auto_focus: true,
      };
      watch!($input.text().clone())
        .subscribe(move |text| {
          *input_value_writer.write() = text.to_string();
        });
      @ { input }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    assert_eq!(*input_value.read(), "");

    wnd.processes_receive_chars("hello\nworld".into());
    wnd.draw_frame();
    assert_eq!(*input_value.read(), "hello world");
  }

  #[test]
  fn input_tap_focus() {
    reset_test_env!();
    let (input_value, input_value_writer) = split_value(String::default());
    let w = fn_widget! {
      let input = @Input { size: None };
      watch!($input.text().clone())
        .subscribe(move |text| {
          *input_value_writer.write() = text.to_string();
        });

      @SizedBox {
        size: Size::new(200., 24.),
        @ { input }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    assert_eq!(*input_value.read(), "");

    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50., 10.).into(),
    });

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);
    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
    wnd.draw_frame();

    wnd.processes_receive_chars("hello".into());
    wnd.draw_frame();
    assert_eq!(*input_value.read(), "hello");
  }
}
