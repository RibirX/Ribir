use std::{collections::HashMap, hash::Hash};

use ribir_core::prelude::*;

use crate::prelude::*;

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
pub struct TextFieldTml<'w> {
  /// Label text is used to inform users as to what information is requested for
  /// a text field.
  label: Option<Label>,

  // /// The placeholder text is displayed in the input field before the user
  // /// enters a value.
  // placeholder: Option<Placeholder>,
  /// Use prefix text before the editable text to show symbols or abbreviations
  /// that help users enter the right type of information in a form’s text
  /// input
  prefix: Option<Leading<Label>>,

  /// Use suffix text after the editable text to show symbols or abbreviations
  /// that help users enter the right type of information in a form’s text
  /// input
  suffix: Option<Trailing<Label>>,

  /// An icon that appears before the editable part of the text field
  leading_icon: Option<Leading<Widget<'w>>>,

  /// An icon that appears after the editable part of the text field
  trailing_icon: Option<Trailing<Widget<'w>>>,
}

#[derive(Clone)]
pub struct TextFieldTheme {
  /// text foreground.
  pub text_brush: Brush,
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
  T: 'static,
  S: Hash + Eq + 'static,
{
  suit: ThemeSuit<S, T>,
  state: S,
}

type TextFieldThemeProxy = ThemeSuitProxy<TextFieldState, TextFieldTheme>;

impl<'c> ComposeChild<'c> for TextFieldThemeProxy {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut child = FatObj::new(child);
      @ $child {
        on_tap: move |_| {
          let mut this = $this.write();
          match this.state {
            TextFieldState::Enabled => this.state = TextFieldState::Focused,
            TextFieldState::Hovered => this.state = TextFieldState::Focused,
            _ => (),
          };
        },
        on_pointer_move: move |_| {
          let mut this = $this.write();
          if this.state == TextFieldState::Enabled { this.state = TextFieldState::Hovered }
        },
        on_pointer_leave: move |_| {
          let mut this = $this.write();
          if this.state == TextFieldState::Hovered { this.state = TextFieldState::Enabled }
        },
        on_focus_out: move |_| {
          let mut this = $this.write();
          if this.state == TextFieldState::Focused { this.state = TextFieldState::Enabled }
        },
      }
    }
    .into_widget()
  }
}

impl TextFieldThemeProxy {
  fn theme(&self) -> Option<&TextFieldTheme> { self.suit.get(self.state) }

  fn label_style(&self, is_text_empty: bool) -> TextStyle {
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
  fn default_style(ctx: &impl AsRef<ProviderCtx>) -> Self {
    Self::from_theme(&Palette::of(ctx), &TypographyTheme::of(ctx))
  }
}

impl TextFieldThemeSuit {
  pub fn from_theme(palette: &Palette, typo_theme: &TypographyTheme) -> Self {
    let body: &TextStyle = &typo_theme.body_large.text;
    let header = &typo_theme.title_large.text;
    let caption = &typo_theme.label_small.text;

    let mut themes = HashMap::new();

    let input_expand_padding = EdgeInsets { left: 16., right: 16., bottom: 16., top: 16. };

    let input_collapse_padding = EdgeInsets { left: 16., right: 16., bottom: 8., top: 8. };

    themes.insert(
      TextFieldState::Enabled,
      TextFieldTheme {
        text_brush: palette.on_surface().into(),
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
        text_brush: palette.on_surface().into(),
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
        text_brush: palette.on_surface().into(),
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

impl<'c> ComposeChild<'c> for TextField {
  type Child = Option<TextFieldTml<'c>>;
  fn compose_child(this: impl StateWriter<Value = Self>, config: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut config = config.unwrap_or_default();
      take_option_field!({leading_icon, trailing_icon}, config);

      let ctx = BuildCtx::get();
      let theme_suit = TextFieldThemeSuit::of(ctx);
      let theme = @TextFieldThemeProxy {
        suit: theme_suit,
        state: TextFieldState::default(),
      }.into_inner();
      let indicator_size = pipe!(Size::new(f32::MAX, $theme.indicator_height));
      let indicator_bg =  pipe!($theme.indicator);
      @Stack {
        @Container {
          size: pipe!(Size::new(0., $theme.container_height)),
          background: pipe!($theme.container_color),
        }
        @Flex {
          justify_content: JustifyContent::Center,
          align_items: Align::Stretch,
          @{
            leading_icon.map(|t| @Icon {
              @{ t.unwrap() }
            })
          }
          @Expanded {
            flex: 1.,
            @{ build_content_area(this, theme, config) }
          }
          @{
            trailing_icon.map(|t| @Icon { @{ t.unwrap() } })
          }
        }
        @Container {
          v_align: VAlign::Bottom,
          size: indicator_size,
          background: indicator_bg,
        }
      }
    }
    .into_widget()
  }
}

fn build_input_area(
  this: impl StateWriter<Value = TextField> + 'static,
  theme: State<TextFieldThemeProxy>,
  prefix: Option<Leading<Label>>,
  suffix: Option<Trailing<Label>>,
  // placeholder: Option<Placeholder>,
) -> Widget<'static> {
  fn_widget! {
    let mut input_area = @Row {
      visible: pipe!(!$this.text.is_empty() || $theme.state == TextFieldState::Focused),
    };
    input_area.get_visibility_widget()
      .map_writer(|w| PartMut::new(&mut w.visible))
      .transition(transitions::LINEAR.of(BuildCtx::get()));

    let mut input = @Input{ };
    $input.write().set_text(&$this.text);

    watch!($input.text().clone())
      .distinct_until_changed()
      .subscribe(move |val| $this.silent().text = val.clone());

    let u = watch!($this.text.clone())
      .distinct_until_changed()
      .subscribe(move |val| $input.write().set_text(&val));

    let h = watch!($theme.state)
      .distinct_until_changed()
      .filter(|state| state == &TextFieldState::Focused)
      .subscribe(move |_| $input.request_focus(FocusReason::Other));
    input.on_disposed(move|_| {
      h.unsubscribe();
      u.unsubscribe();
    });

    @Row {
      @{
        prefix.map(|p| @Text{
          text: p.unwrap().0,
          foreground: pipe!($theme.text_brush.clone()),
          text_style: pipe!($theme.text.clone()),
        })
      }
      @Expanded {
        flex: 1.,
        @ $input { }
      }
      @{
        suffix.map(|s| @Text{
          text: s.unwrap().0,
          foreground: pipe!($theme.text_brush.clone()),
          text_style: pipe!($theme.text.clone()),
        })
      }
    }
  }
  .into_widget()
}

#[derive(Declare)]
struct TextFieldLabel {
  text: CowArc<str>,
  style: TextStyle,
}

impl Compose for TextFieldLabel {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let label = @Text {
        v_align: VAlign::Top,
        text: pipe!($this.text.clone()),
        text_style: pipe!($this.style.clone()),
      };

      this.map_writer(|w| PartMut::new(&mut w.style.font_size))
        .transition(transitions::LINEAR.of(BuildCtx::get()));

      label
    }
    .into_widget()
  }
}

fn build_content_area(
  this: impl StateWriter<Value = TextField> + 'static, theme: State<TextFieldThemeProxy>,
  mut config: TextFieldTml,
) -> Widget<'static> {
  fn_widget! {
    take_option_field!({label, prefix, suffix}, config);
    let mut content_area = @Column {
      padding: pipe!($theme.input_padding($this.text.is_empty())),
    };

    part_writer!(&mut content_area.padding)
      .transition(transitions::LINEAR.of(BuildCtx::get()));

    @ $content_area {
      @ {
        label.map(|label| @Expanded {
          flex: 1.,
          @TextFieldLabel {
            text: label.0,
            style: pipe!($theme.label_style($this.text.is_empty())),
          }
        })
      }
      @ { build_input_area(this, theme, prefix, suffix)}
    }
  }
  .into_widget()
}
