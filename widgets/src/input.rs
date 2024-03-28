use ribir_core::{prelude::*, ticker::FrameMsg};
mod caret;
mod caret_state;
mod glyphs_helper;
mod handle;
mod selected_text;
mod text_selectable;
use std::{ops::Range, rc::Rc};

pub use caret_state::{CaretPosition, CaretState};
pub use selected_text::SelectedHighLightStyle;
pub use text_selectable::TextSelectable;

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

struct ImeHandle<H> {
  host: H,
  pre_edit: Option<PreEditState>,
  guard: Option<SubscriptionGuard<BoxSubscription<'static>>>,
  window: Rc<Window>,
  caret_id: LazyWidgetId,
}

impl<E, H> ImeHandle<H>
where
  E: EditableText + 'static,
  H: StateWriter<Value = E>,
{
  fn new(window: Rc<Window>, host: H, caret_id: LazyWidgetId) -> Self {
    Self { window, host, pre_edit: None, guard: None, caret_id }
  }
  fn ime_allowed(&mut self) {
    self.window.set_ime_allowed(true);
    self.track_cursor();
  }

  fn ime_disallowed(&mut self) {
    self.window.set_ime_allowed(false);
    self.guard = None;
  }

  fn update_pre_edit(&mut self, e: &ImePreEditEvent) {
    match &e.pre_edit {
      ImePreEdit::Begin => {
        let mut host = self.host.write();
        let rg = host.caret().select_range();
        host.writer().delete_byte_range(&rg);
        self.pre_edit = Some(PreEditState { position: rg.start, value: None });
      }
      ImePreEdit::PreEdit { value, cursor } => {
        let Some(PreEditState { position, value: edit_value }) = self.pre_edit.as_mut() else {
          return;
        };
        let mut host = self.host.write();
        let mut writer = host.writer();
        if let Some(txt) = edit_value {
          writer.delete_byte_range(&(*position..*position + txt.len()));
        }
        writer.insert_str(value);
        writer.set_to(*position + cursor.map_or(0, |(start, _)| start));
        *edit_value = Some(value.clone());
      }
      ImePreEdit::End => {
        if let Some(PreEditState { value: Some(txt), position, .. }) = self.pre_edit.take() {
          let mut host = self.host.write();
          let mut writer = host.writer();
          writer.delete_byte_range(&(position..position + txt.len()));
        }
      }
    }
    if self.pre_edit.is_none() {
      self.track_cursor();
    } else {
      self.guard = None;
    }
  }

  fn track_cursor(&mut self) {
    if self.guard.is_some() {
      return;
    }

    let window = self.window.clone();
    let wid = self.caret_id.clone();

    let pos = window.map_to_global(Point::zero(), wid.assert_id());
    let size = window
      .layout_size(wid.assert_id())
      .unwrap_or_default();
    window.set_ime_cursor_area(&Rect::new(pos, size));

    let tick_of_layout_ready = window
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    self.guard = Some(
      self
        .host
        .modifies()
        .sample(tick_of_layout_ready)
        .box_it()
        .subscribe(move |_| {
          let pos = window.map_to_global(Point::zero(), wid.assert_id());
          let size = window
            .layout_size(wid.assert_id())
            .unwrap_or_default();
          window.set_ime_cursor_area(&Rect::new(pos, size));
        })
        .unsubscribe_when_dropped(),
    );
  }
}

impl ComposeChild for Input {
  type Child = Option<State<Placeholder>>;
  fn compose_child(
    this: impl StateWriter<Value = Self>, placeholder: Self::Child,
  ) -> impl WidgetBuilder {
    fn_widget! {
      let text = @Text {
        text: pipe!($this.text.clone()),
        text_style: pipe!($this.style.clone()),
      };
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
    this: impl StateWriter<Value = Self>, placeholder: Self::Child,
  ) -> impl WidgetBuilder {
    fn_widget! {
      let text = @Text {
        text: pipe!($this.text.clone()),
        text_style: pipe!($this.style.clone()),
        overflow: pipe!($this.auto_wrap).map(|auto_wrap| match auto_wrap {
          true => Overflow::AutoWrap,
          false => Overflow::Clip,
        }),
      };

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
    this: &impl StateWriter<Value = Self>, mut text: FatObj<State<Text>>,
    scroll_dir: impl Pipe<Value = Scrollable> + 'static, placeholder: Option<State<Placeholder>>,
  ) -> impl WidgetBuilder {
    fn_widget! {
      let layout_box = text.get_layout_box_widget().clone_reader();
      let only_text = text.clone_reader();

      let mut stack = @Stack {
        fit: StackFit::Passthrough,
        scrollable: scroll_dir,
      };

      let caret_box = @ConstrainedBox {
        clamp: pipe!(
            $this.current_line_height(&$text, $text.layout_size()).unwrap_or(0.)
          ).map(BoxClamp::fixed_height),
      };

      let caret_box_id = caret_box.lazy_host_id();
      let mut caret_box = @$caret_box {
        anchor: pipe!(
          let pos = $this.caret_position(&$text, $text.layout_size()).unwrap_or_default();
          Anchor::left_top(pos.x, pos.y)
        ),
      };
      let scrollable = stack.get_scrollable_widget();
      let tick_of_layout_ready = ctx!().window()
        .frame_tick_stream()
        .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
      watch!(SelectableText::caret(&*$this))
        .distinct_until_changed()
        .sample(tick_of_layout_ready)
        .map(move |_| $this.caret_position(&$text, $text.layout_size()).unwrap_or_default())
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

      let ime_handle = Stateful::new(
        ImeHandle::new(ctx!().window(), this.clone_writer(), caret_box_id)
      );
      let mut stack = @ $stack {
        on_focus: move |_| $ime_handle.write().ime_allowed(),
        on_blur: move |_| $ime_handle.write().ime_disallowed(),
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
          $ime_handle.write().update_pre_edit(e);
        },
      };

      let high_light_rect = @UnconstrainedBox {
        clamp_dim: ClampDim::MIN_SIZE,
        @OnlySizedByParent {
          @SelectedHighLight {
            visible: pipe!($stack.has_focus()),
            rects: pipe! { $this.select_text_rect(&$text, $text.layout_size()) }
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

      let text_widget = text.build(ctx!());
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
  let mut clamp: BoxClamp =
    BoxClamp { min: Size::new(0., 0.), max: Size::new(f32::INFINITY, f32::INFINITY) };
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
  use ribir_core::{
    prelude::*,
    reset_test_env,
    test_helper::{split_value, TestWindow},
  };
  use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

  use super::{EditableText, Input};
  use crate::layout::SizedBox;

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
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (50., 10.).into() });

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);
    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
    wnd.draw_frame();

    wnd.processes_receive_chars("hello".into());
    wnd.draw_frame();
    assert_eq!(*input_value.read(), "hello");
  }
}
