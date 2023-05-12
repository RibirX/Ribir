mod caret;
mod caret_state;
mod editarea;
mod glyphs_helper;
mod handle;
mod selected_text;
use std::time::Duration;

pub use caret_state::CaretState;

pub use self::editarea::PlaceholderStyle;
use self::editarea::TextEditorArea;
pub use self::selected_text::SelectedTextStyle;
use crate::layout::Container;
use ribir_core::prelude::*;

pub struct Placeholder(CowArc<str>);

impl Placeholder {
  #[inline]
  pub fn new(str: impl Into<CowArc<str>>) -> Self { Self(str.into()) }
}
#[derive(Clone, PartialEq)]
pub struct InputStyle {
  pub size: f32,
}
impl CustomStyle for InputStyle {}

#[derive(Declare)]
pub struct Input {
  #[declare(default = TypographyTheme::of(ctx).body_large.text.clone())]
  pub style: CowArc<TextStyle>,
  #[declare(skip)]
  text: CowArc<str>,
  #[declare(skip)]
  caret: CaretState,
  #[declare(default = InputStyle::of(ctx).size)]
  size: f32,
}

#[derive(Declare)]
pub struct TextArea {
  #[declare(default = TypographyTheme::of(ctx).body_large.text.clone())]
  pub style: CowArc<TextStyle>,
  #[declare(default = true)]
  pub auto_wrap: bool,
  #[declare(skip)]
  text: CowArc<str>,
  #[declare(skip)]
  caret: CaretState,
  #[declare(default = TextAreaStyle::of(ctx).rows)]
  rows: f32,
  #[declare(default = TextAreaStyle::of(ctx).cols)]
  cols: f32,
}

impl Input {
  pub fn text(&self) -> CowArc<str> { self.text.clone() }

  pub fn caret(&self) -> &CaretState { &self.caret }

  pub fn set_text(&mut self, text: impl Into<CowArc<str>>) {
    self.text = text.into();
    self.caret.valid(self.text.len());
  }

  pub fn set_caret(&mut self, caret: CaretState) {
    self.caret = caret;
    self.caret.valid(self.text.len());
  }
}

impl TextArea {
  pub fn text(&self) -> CowArc<str> { self.text.clone() }

  pub fn caret(&self) -> &CaretState { &self.caret }

  pub fn set_text(&mut self, text: impl Into<CowArc<str>>) {
    self.text = text.into();
    self.caret.valid(self.text.len());
  }

  pub fn set_caret(&mut self, caret: CaretState) {
    self.caret = caret;
    self.caret.valid(self.text.len());
  }
}

impl ComposeChild for Input {
  type Child = Option<Placeholder>;
  fn compose_child(this: State<Self>, placeholder: Self::Child) -> Widget {
    widget! {
      init ctx => {
        let frame_scheduler = ctx.wnd_ctx().frame_scheduler();
      }
      states {
        this: this.into_writable(),
      }
      Container {
        size: Size::new(
          this.size * glyph_width(this.style.font_size),
          line_height(this.style.line_height.unwrap_or(this.style.font_size.into_em()))
        ),
        TextEditorArea {
          id: area,
          text: this.text.clone(),
          style: this.style.clone(),
          caret: this.caret().clone(),
          multi_line: false,
          auto_wrap: false,

          widget::from(placeholder)
        }
      }
      finally {
        let_watch!(area.clone_stateful())
          .delay(Duration::ZERO, frame_scheduler)
          .subscribe(move |area| {
            let area = area.state_ref();
            if area.caret != this.caret {
              this.silent().caret = area.caret.clone();
            }
            if area.text != this.text {
              this.silent().text = area.text.clone();
            }
          });
      }
    }
    .into_widget()
  }
}

#[derive(Clone, PartialEq)]
pub struct TextAreaStyle {
  pub rows: f32,
  pub cols: f32,
}
impl CustomStyle for TextAreaStyle {}

impl ComposeChild for TextArea {
  type Child = Option<Placeholder>;
  fn compose_child(this: State<Self>, placeholder: Self::Child) -> Widget {
    widget! {
      init ctx => {
        let frame_scheduler = ctx.wnd_ctx().frame_scheduler();
      }
      states {
        this: this.into_writable(),
      }
      Container {
        size: Size::new(
          this.cols * glyph_width(this.style.font_size),
          this.rows * line_height(this.style.line_height.unwrap_or(this.style.font_size.into_em()))
        ),
        TextEditorArea {
          id: area,
          text: this.text.clone(),
          style: this.style.clone(),
          caret: this.caret.clone(),
          multi_line: true,
          auto_wrap: no_watch!(this.auto_wrap),

          widget::from(placeholder)
        }
      }
      finally {
        let_watch!(area.clone_stateful())
          .delay(Duration::ZERO, frame_scheduler)
          .subscribe(move |area| {
            let area = area.state_ref();
            if area.caret != this.caret {
              this.silent().caret = area.caret.clone();
            }
            if area.text != this.text {
              this.silent().text = area.text.clone();
            }
          });
      }
    }
    .into_widget()
  }
}

fn glyph_width(font_size: FontSize) -> f32 {
  FontSize::Em(font_size.relative_em(1.)).into_pixel().value()
}

fn line_height(line_height: Em) -> f32 { FontSize::Em(line_height).into_pixel().value() }

pub fn add_to_theme(theme: &mut FullTheme) {
  theme
    .custom_styles
    .set_custom_style(InputStyle { size: 20. });
  theme
    .custom_styles
    .set_custom_style(TextAreaStyle { rows: 2., cols: 20. });
  theme.custom_styles.set_custom_style(SelectedTextStyle {
    brush: Color::from_rgb(181, 215, 254).into(),
  });
  theme.custom_styles.set_custom_style(PlaceholderStyle {
    foreground: theme.palette.on_surface_variant().into(),
    text_style: theme.typography_theme.body_medium.text.clone(),
  });
}
