use crate::{
  layout::{Column, Container},
  prelude::{ExpandBox, ExpandDir, Expanded, Icon, Input, Row, Stack, Text},
};
use ribir_core::prelude::*;
use std::hash::Hash;
use std::{collections::HashMap, time::Duration};

#[derive(Declare, Default)]
pub struct TextField {
  /// textfield's input value
  #[declare(default)]
  text: String,
}

/// Label text is used to inform users as to what information is requested for a text field.
pub struct LabelStr(pub String);

/// The placeholder text is displayed in the input field before the user enters a value.
pub struct PlaceholderStr(pub String);

/// Use prefix text before the editable text to show symbols or abbreviations that help users 
/// enter the right type of information in a form’s text input
pub struct PrefixStr(pub String);

/// Use suffix text after the editable text to show symbols or abbreviations that help users 
/// enter the right type of information in a form’s text input
pub struct SubfixStr(pub String);

/// An icon that appears before the editable part of the text field

pub struct LeadingIcon(pub Widget);

/// An icon that appears after the editable part of the text field
pub struct TrailingIcon(pub Widget);

#[derive(Template)]
pub struct TextFieldConfig {
  label: Option<LabelStr>,
  placeholder: Option<PlaceholderStr>,
  prefix: Option<PrefixStr>,
  subfix: Option<SubfixStr>,
  leading_icon: Option<LeadingIcon>,
  trailing_icon: Option<TrailingIcon>,
}

#[derive(Clone)]
pub struct TextFieldTheme {
  /// textfield input's text style
  pub text: TextStyle,

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
  pub label_collapse: TextStyle,

  /// label's text style when expand
  pub label_expand: TextStyle,

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
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget
  where
    Self: Sized,
  {
    widget! {
      track {this: this.into_stateful()}
      DynWidget {
        dyns: {
          child
        },
        focus_in: move |_| {
          match this.state {
            TextFieldState::Enabled => this.state = TextFieldState::Focused,
            TextFieldState::Hovered => this.state = TextFieldState::Focused,
            _ => (),
          };
        },

        pointer_move: move |_| {
          match this.state {
            TextFieldState::Enabled => this.state = TextFieldState::Hovered,
            _ => (),
          };
        },

        pointer_leave: move |_| {
          match this.state {
            TextFieldState::Hovered => this.state = TextFieldState::Enabled,
            _ => (),
          };
        },
        focus_out: move |_| {
          match this.state {
            TextFieldState::Focused => this.state = TextFieldState::Enabled,
            _ => (),
          };
        },
      }
    }
  }
}

impl TextFieldThemeProxy {
  fn theme(&self) -> Option<&TextFieldTheme> { self.suit.get(self.state) }

  fn label_style(&self, is_text_empty: bool) -> TextStyle {
    let mut font = if self.is_collapse(is_text_empty) {
        self.label_collapse.clone()
      } else {
        self.label_expand.clone()
      };
    font.foreground = Brush::Color(self.label_color);
    font
  }

  fn input_padding(&self, is_text_empty: bool) -> EdgeInsets {
    if self.is_collapse(is_text_empty) {
      self.input_collapse_padding.clone()
    } else {
      self.input_expand_padding.clone()
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


impl CustomTheme for TextFieldThemeSuit {}

impl TextFieldThemeSuit {
  pub fn from_theme(palette: &Palette, typo_theme: &TypographyTheme) -> Self {
    let body = typo_theme.body1.text.clone();
    let header = typo_theme.headline6.text.clone();
    let caption = typo_theme.caption.text.clone();

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
        text: body.clone(),
        container_color: palette.surface_variant(),
        indicator: palette.on_surface_variant(),
        indicator_height: 1.,
        label_color: palette.on_surface_variant(),

        container_height: 56.,
        label_collapse: caption.clone(),
        label_expand: header.clone(),
        input_collapse_padding: input_collapse_padding.clone(),
        input_expand_padding: input_expand_padding.clone(),
      },
    );

    themes.insert(
      TextFieldState::Focused,
      TextFieldTheme {
        text: body.clone(),
        container_color: palette.surface_variant(),
        indicator: palette.primary(),
        indicator_height: 2.,
        label_color: palette.primary(),

        container_height: 56.,
        label_collapse: caption.clone(),
        label_expand: header.clone(),
        input_collapse_padding: input_collapse_padding.clone(),
        input_expand_padding: input_expand_padding.clone(),
      },
    );

    themes.insert(
      TextFieldState::Hovered,
      TextFieldTheme {
        text: body.clone(),
        container_color: palette.surface_variant(),
        indicator: palette.on_surface(),
        indicator_height: 2.,
        label_color: palette.on_surface(),

        container_height: 56.,
        label_collapse: caption.clone(),
        label_expand: header.clone(),
        input_collapse_padding: input_collapse_padding.clone(),
        input_expand_padding: input_expand_padding.clone(),
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
  type Child = TextFieldConfig;
  fn compose_child(this: StateWidget<Self>, mut config: Self::Child) -> Widget
  where
    Self: Sized,
  {
    widget! {
      track{
        this: this.into_stateful()
      }
      env {
        take_option_field!({leading_icon, trailing_icon}, config);
      }

      TextFieldThemeProxy {
        id: theme,
        suit: TextFieldThemeSuit::of(ctx).clone(),
        state: TextFieldState::default(),

        Container {
          size: Size::new(f32::MAX, theme.container_height),
          background: theme.container_color.clone(),
          Stack {
            Row {
              ExpandBox {
                dir: ExpandDir::Y,
                DynWidget {
                  v_align: VAlign::Center,
                  dyns: build_icon(leading_icon.map(|l| l.0))
                }
              }
              Expanded {
                flex: 1.,
                DynWidget {
                  dyns: move |_: &BuildCtx| build_content_area(&mut this, &mut theme, config)
                }
              }
              ExpandBox {
                dir: ExpandDir::Y,
                DynWidget {
                  v_align: VAlign::Center,
                  dyns: build_icon(trailing_icon.map(|t| t.0))
                }
              }
            }

            Container {
              v_align: VAlign::Bottom,
              size: Size::new(f32::MAX, theme.indicator_height),
              background: theme.indicator.clone(),
            }
          }
        }
      }
    }
  }
}

fn build_input_area(
  this: &mut StateRef<TextField>,
  theme: &mut StateRef<TextFieldThemeProxy>,
  prefix: Option<PrefixStr>,
  subfix: Option<SubfixStr>,
  placeholder: Option<PlaceholderStr>,
) -> Widget {
  widget! {
    track { this: this.clone_stateful(), theme: theme.clone_stateful(), }
    Row {
      id: input_area,
      visible: !this.text.is_empty() || theme.state == TextFieldState::Focused,
      DynWidget {
        dyns: prefix.map(|text| {
          Text {
            text: text.0.clone().into(),
            style: theme.text.clone(),
          }
        })
      }

      Expanded {
        flex: 1.,
        Input {
          id: input,
          text:  this.text.clone(),
          style: theme.text.clone(),
          placeholder: placeholder.map(|p| p.0.clone()) ,
        }
      }
      DynWidget {
        dyns: subfix.map(|text| {
          Text {
            text: text.0.clone().into(),
            style: theme.text.clone(),
          }
        })
      }
    }
    change_on input_area.visible Animate {
      id: input_animate,
      from: State {
        input_area.visible: false,
      },
      transition: Transition {
        duration: Duration::from_millis(1),
        easing: easing::steps(1, easing::StepsJump::JumpStart),
        delay: Duration::from_millis(400),
      }
    }

    on input.text.clone() {
      change: move |(_, val)| {
        this.silent().text = val;
      }
    }
  }
}

#[derive(Declare)]
struct TextFieldLabel {
  text: String,
  style: TextStyle,
}

impl Compose for TextFieldLabel {
  fn compose(this: StateWidget<Self>) -> Widget
  where
    Self: Sized,
  {
    widget_try_track! {
      try_track { this }
      Text {
        id: label,
        v_align: VAlign::Top,
        text: this.text.clone(),
        style: this.style.clone(),
      }

      change_on label.style Animate {
        transition: Transition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(500)
        },
        lerp_fn: move |from, to, rate| {
          let from_size = from.font_size.into_pixel();
          let to_size = to.font_size.into_pixel();

          let mut res = to.clone();
          res.font_size = FontSize::Pixel(Pixel(from_size.0.lerp(&to_size.0, rate).into()));
          res
        }
      }
    }
  }
}


fn build_content_area(
  this: &mut StateRef<TextField>,
  theme: &mut StateRef<TextFieldThemeProxy>,
  mut config: TextFieldConfig,
) -> Widget {
  widget! {
    track { this: this.clone_stateful(), theme: theme.clone_stateful(), }
    env {
      take_option_field!({label, prefix, subfix, placeholder}, config);
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

      DynWidget {
        dyns: move |_: &BuildCtx| build_input_area(&mut this, &mut theme, prefix, subfix, placeholder)
      }
    }

    change_on content_area.padding Transition {
      duration: Duration::from_millis(500),
      easing: easing::LINEAR,
    }
  }
}

fn build_icon(icon: Option<Widget>) -> Widget {
  if icon.is_some() {
    widget! {
      Icon {
        size: IconSize::of(ctx).small,
        DynWidget {
          dyns: icon.unwrap()
        }
      }
    }
  } else {
    Void {}.into_widget()
  }
}
