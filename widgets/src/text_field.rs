use crate::{
  common_widget::{Leading, LeadingText, Trailing, TrailingText},
  input::Placeholder,
  layout::{Column, ConstrainedBox, Container},
  prelude::{Expanded, Icon, Input, Label, Row, Stack, Text},
};
use ribir_core::prelude::*;
use std::{collections::HashMap, hash::Hash, ops::Deref};

#[derive(Declare, Default)]
pub struct TextField {
  /// textfield's input value
  #[declare(skip)]
  text: CowArc<str>,
}

impl TextField {
  pub fn text(&self) -> CowArc<str> { self.text.clone() }
  pub fn set_text(&mut self, text: CowArc<str>) { self.text = text; }
}

#[derive(Template, Default)]
pub struct TextFieldTml {
  /// Label text is used to inform users as to what information is requested for
  /// a text field.
  label: Option<Label>,

  /// The placeholder text is displayed in the input field before the user
  /// enters a value.
  placeholder: Option<Placeholder>,

  /// Use prefix text before the editable text to show symbols or abbreviations
  /// that help users enter the right type of information in a form’s text
  /// input
  prefix: Option<LeadingText>,

  /// Use suffix text after the editable text to show symbols or abbreviations
  /// that help users enter the right type of information in a form’s text
  /// input
  suffix: Option<TrailingText>,

  /// An icon that appears before the editable part of the text field
  leading_icon: Option<WidgetOf<Leading>>,

  /// An icon that appears after the editable part of the text field
  trailing_icon: Option<WidgetOf<Trailing>>,
}

#[derive(Clone)]
pub struct TextFieldTheme {
  /// text foreground.
  pub foreground: Brush,
  /// textfield input's text style
  pub text: CowArc<TextStyle>,

  /// textfield's background color
  pub container_color: Color,

  /// textfield component's height
  pub container_height: f32,

  /// indicator's color
  pub indicator: Color,
  pub indicator_height: f32,

  /// label text color
  pub label_color: Color,

  /// label's text style when collapse
  pub label_collapse: CowArc<TextStyle>,

  /// label's text style when expand
  pub label_expand: CowArc<TextStyle>,

  /// edit area's padding when collapse
  pub input_collapse_padding: EdgeInsets,

  /// edit area's padding when expand
  pub input_expand_padding: EdgeInsets,
}

#[derive(Clone)]
pub struct ThemeSuit<S, T>
where
  S: Hash + Eq,
{
  themes: HashMap<S, T>,
}

impl<S, T> ThemeSuit<S, T>
where
  S: Hash + Eq,
{
  fn get(&self, state: S) -> Option<&T> { self.themes.get(&state) }
}

#[derive(Declare)]
struct ThemeSuitProxy<S, T>
where
  S: Hash + Eq,
{
  suit: ThemeSuit<S, T>,
  state: S,
}

type TextFieldThemeProxy = ThemeSuitProxy<TextFieldState, TextFieldTheme>;

impl ComposeChild for TextFieldThemeProxy {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget
  where
    Self: Sized,
  {
    widget! {
      states {this: this.into_writable()}
      DynWidget {
        dyns: {
          child
        },
        on_tap: move |_| {
          match this.state {
            TextFieldState::Enabled => this.state = TextFieldState::Focused,
            TextFieldState::Hovered => this.state = TextFieldState::Focused,
            _ => (),
          };
        },

        on_pointer_move: move |_| {
          if this.state == TextFieldState::Enabled { this.state = TextFieldState::Hovered }
        },

        on_pointer_leave: move |_| {
          if this.state == TextFieldState::Hovered { this.state = TextFieldState::Enabled }
        },
        on_focus_out: move |_| {
          if this.state == TextFieldState::Focused { this.state = TextFieldState::Enabled }
        },
      }
    }
    .into()
  }
}

impl TextFieldThemeProxy {
  fn theme(&self) -> Option<&TextFieldTheme> { self.suit.get(self.state) }

  fn label_style(&self, is_text_empty: bool) -> CowArc<TextStyle> {
    if self.is_collapse(is_text_empty) {
      self.label_collapse.clone()
    } else {
      self.label_expand.clone()
    }
  }

  fn input_padding(&self, is_text_empty: bool) -> EdgeInsets {
    if self.is_collapse(is_text_empty) {
      self.input_collapse_padding
    } else {
      self.input_expand_padding
    }
  }

  fn is_collapse(&self, is_text_empty: bool) -> bool {
    !is_text_empty || self.state == TextFieldState::Focused
  }
}

pub type TextFieldThemeSuit = ThemeSuit<TextFieldState, TextFieldTheme>;

impl Deref for TextFieldThemeProxy {
  type Target = TextFieldTheme;
  fn deref(&self) -> &Self::Target { self.theme().unwrap() }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub enum TextFieldState {
  #[default]
  Enabled,
  Focused,
  Hovered,
  // Disabled,
}

impl CustomStyle for TextFieldThemeSuit {
  fn default_style(ctx: &BuildCtx) -> Self {
    Self::from_theme(Palette::of(ctx), TypographyTheme::of(ctx))
  }
}

impl TextFieldThemeSuit {
  pub fn from_theme(palette: &Palette, typo_theme: &TypographyTheme) -> Self {
    let body: &CowArc<TextStyle> = &typo_theme.body_large.text;
    let header = &typo_theme.title_large.text;
    let caption = &typo_theme.label_small.text;

    let mut themes = HashMap::new();

    let input_expand_padding = EdgeInsets {
      left: 16.,
      right: 16.,
      bottom: 16.,
      top: 16.,
    };

    let input_collapse_padding = EdgeInsets {
      left: 16.,
      right: 16.,
      bottom: 8.,
      top: 8.,
    };

    themes.insert(
      TextFieldState::Enabled,
      TextFieldTheme {
        foreground: palette.on_surface().into(),
        text: body.clone(),
        container_color: palette.surface_variant(),
        indicator: palette.on_surface_variant(),
        indicator_height: 1.,
        label_color: palette.on_surface_variant(),

        container_height: 56.,
        label_collapse: caption.clone(),
        label_expand: header.clone(),
        input_collapse_padding,
        input_expand_padding,
      },
    );

    themes.insert(
      TextFieldState::Focused,
      TextFieldTheme {
        foreground: palette.on_surface().into(),
        text: body.clone(),
        container_color: palette.surface_variant(),
        indicator: palette.primary(),
        indicator_height: 2.,
        label_color: palette.primary(),

        container_height: 56.,
        label_collapse: caption.clone(),
        label_expand: header.clone(),
        input_collapse_padding,
        input_expand_padding,
      },
    );

    themes.insert(
      TextFieldState::Hovered,
      TextFieldTheme {
        foreground: palette.on_surface().into(),
        text: body.clone(),
        container_color: palette.surface_variant(),
        indicator: palette.on_surface(),
        indicator_height: 2.,
        label_color: palette.on_surface(),

        container_height: 56.,
        label_collapse: caption.clone(),
        label_expand: header.clone(),
        input_collapse_padding,
        input_expand_padding,
      },
    );

    // themes.insert(
    //   TextFieldState::Disabled,
    //   TextFieldTheme {
    //     text: body.clone(),
    //     container_color: palette.on_surface(),
    //     indicator: palette.on_surface(),
    //     indicator_height: 2.,
    //     label_color: palette.on_surface(),

    //     container_height: 56.,
    //     label_collapse: caption.clone(),
    //     label_expand: header.clone(),
    //     input_collapse_padding: input_collapse_padding.clone(),
    //     input_expand_padding: input_expand_padding.clone(),
    //   },
    // );
    Self { themes }
  }
}

macro_rules! take_option_field {
  ({$($f: ident),+}, $c: ident) => {
    $(let $f = $c.$f.take();)+
  }
}

impl ComposeChild for TextField {
  type Child = Option<TextFieldTml>;
  fn compose_child(this: State<Self>, config: Self::Child) -> Widget
  where
    Self: Sized,
  {
    let mut config = config.unwrap_or_default();
    widget! {
      states {
        this: this.into_writable(),
      }
      init ctx => {
        let theme_suit = TextFieldThemeSuit::of(ctx).clone();
      }
      init {
        take_option_field!({leading_icon, trailing_icon}, config);
      }

      TextFieldThemeProxy {
        id: theme,
        suit: theme_suit,
        state: TextFieldState::default(),

          Stack {
            Container {
              size: Size::new(0., theme.container_height),
              background: theme.container_color,
            }
            Row {
              ConstrainedBox {
                clamp: BoxClamp::EXPAND_Y,
                DynWidget {
                  v_align: VAlign::Center,
                  dyns: build_icon(leading_icon.map(|l| l.child))
                }
              }
              Expanded {
                flex: 1.,
                build_content_area(no_watch!(&mut this), no_watch!(&mut theme), config)
              }
              ConstrainedBox {
                clamp: BoxClamp::EXPAND_Y,
                DynWidget {
                  v_align: VAlign::Center,
                  dyns: build_icon(trailing_icon.map(|t| t.child))
                }
              }
            }

            Container {
              v_align: VAlign::Bottom,
              size: Size::new(f32::MAX, theme.indicator_height),
              background: theme.indicator,
            }
          }
      }
    }
    .into()
  }
}

fn build_input_area(
  this: &mut StateRef<TextField>,
  theme: &mut StateRef<TextFieldThemeProxy>,
  prefix: Option<LeadingText>,
  suffix: Option<TrailingText>,
  placeholder: Option<Placeholder>,
) -> Widget {
  fn text_label(text: CowArc<str>, theme: StateRef<TextFieldThemeProxy>) -> Text {
    Text {
      text,
      foreground: theme.foreground.clone(),
      text_style: theme.text.clone(),
      path_style: PathPaintStyle::Fill,
      overflow: Overflow::Clip,
    }
  }

  widget! {
    states { this: this.clone_stateful(), theme: theme.clone_stateful() }
    init ctx => {
      let linear = transitions::LINEAR.of(ctx);
      let prefix = prefix.map(move |p| p.child);
      let suffix = suffix.map(move|s| s.child);
    }
    Row {
      id: input_area,
      visible: !this.text.is_empty() || theme.state == TextFieldState::Focused,
      Option::map(prefix.clone(), move |text| text_label(text, theme))
      Expanded {
        flex: 1.,
        Input {
          id: input,
          style: theme.text.clone(),
          widget::from(placeholder)
        }
      }
      Option::map(suffix.clone(),   move |text| text_label(text, theme))

    }
    transition prop!(input_area.visible, move |_from, to, rate| *to && rate >= 1.) {
      by: linear,
    }

    finally {
      input.set_text(this.text.clone());
      let_watch!(input.text())
        .distinct_until_changed()
        .subscribe(move |val| {
          this.silent().text = val;
        });
      let_watch!(this.text.clone())
        .distinct_until_changed()
        .subscribe(move |val| input.set_text(val));
      let_watch!(theme.state)
        .distinct_until_changed()
        .subscribe(move |state| {
          if state == TextFieldState::Focused {
            input.request_focus();
          }
        });
    }
  }
  .into()
}

#[derive(Declare)]
struct TextFieldLabel {
  text: CowArc<str>,
  style: CowArc<TextStyle>,
}

impl Compose for TextFieldLabel {
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      init ctx => {
        let linear = transitions::LINEAR.of(ctx);
      }
      Text {
        id: label,
        v_align: VAlign::Top,
        text: this.text.clone(),
        text_style: this.style.clone(),
      }

      // todo: prop with inner field's property
      // transition prop!(label.style.font_size) {
      //   by: transitions::LINEAR.of(ctx)
      // }
      transition prop!(label.text_style, move |from, to, rate| {
        let from_size = from.font_size.into_pixel();
        let to_size = to.font_size.into_pixel();

        let mut res = to.clone();
        res.to_mut().font_size = FontSize::Pixel(Pixel(from_size.0.lerp(&to_size.0, rate).into()));
        res
      }) {
        by: linear,
      }
    }
    .into()
  }
}

fn build_content_area(
  this: &mut StateRef<TextField>,
  theme: &mut StateRef<TextFieldThemeProxy>,
  mut config: TextFieldTml,
) -> Widget {
  widget! {
    states { this: this.clone_stateful(), theme: theme.clone_stateful(), }
    init ctx => {
      let linear = transitions::LINEAR.of(ctx);
    }
    init {
      take_option_field!({label, prefix, suffix, placeholder}, config);
    }
    Column {
      id: content_area,
      padding: theme.input_padding(this.text.is_empty()),

      DynWidget {
        dyns: label.map(move |label| {
          widget! {
            Expanded {
              flex: 1.,
              TextFieldLabel {
                text: label.0.clone(),
                style: theme.label_style(this.text.is_empty()),
              }
            }
          }
        })
      }
      build_input_area(
        no_watch!(&mut this),
        no_watch!(&mut theme),
        prefix,
        suffix,
        placeholder
      )
    }

    transition prop!(content_area.padding) { by: linear }
  }
  .into()
}

fn build_icon(icon: Option<Widget>) -> Widget {
  if icon.is_some() {
    widget! {
      init ctx => {
        let icon_size = IconSize::of(ctx).small;
      }
      Icon {
        size: icon_size,
        DynWidget {
          dyns: icon.unwrap()
        }
      }
    }
    .into()
  } else {
    Void.into()
  }
}
