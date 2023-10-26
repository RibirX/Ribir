use crate::wordle::{CharHint, Wordle, WordleChar};
use ribir::prelude::*;

pub fn wordle_game() -> impl WidgetBuilder {
  fn_widget! { @ { Wordle::new(5, 5) }  }
}

trait WordleExtraWidgets: StateWriter<Value = Wordle> + Sized {
  fn chars_key<const N: usize>(
    &self,
    chars: [char; N],
  ) -> impl Iterator<Item = impl WidgetBuilder> {
    chars.into_iter().map(|c| self.char_key(c))
  }

  fn char_key(&self, key: char) -> impl WidgetBuilder {
    let this = self.clone_writer();
    fn_widget! {
      @FilledButton {
        on_tap: move |_| $this.write().guessing.enter_char(key),
        color: pipe!{ hint_color($this.key_hint(key), ctx!()) },
        @ { Label::new(key.to_string()) }
      }
    }
  }

  fn keyboard(&self, state_bar: impl StateWriter<Value = Text>) -> impl WidgetBuilder {
    let this = self.clone_writer();
    fn_widget! {
    let palette = Palette::of(ctx!());
    @Column {
        main_axis_gap: 5.,
        align_items: Align::Center,
        justify_content: JustifyContent::Center,
        @Row {
          main_axis_gap: 5.,
          align_items: Align::Center,
          justify_content: JustifyContent::Center,
          @ { self.chars_key(['Q', 'W', 'E', 'R','T', 'Y', 'U', 'I','O', 'P']) }
        }
        @Row {
          main_axis_gap: 5.,
          align_items: Align::Center,
          justify_content: JustifyContent::Center,
          @ { self.chars_key(['A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L' ]) }
        }
        @Row {
          main_axis_gap: 5.,
          align_items: Align::Center,
          justify_content: JustifyContent::Center,
          @FilledButton {
            on_tap: move |_| $this.write().guessing.delete_back_char(),
            color: palette.surface_variant(),
            @ { Label::new("Del") }
          }
          @ { self.chars_key(['Z', 'X', 'C', 'V', 'B', 'N', 'M' ]) }

          @FilledButton {
            on_tap: move |_| match $this.write().guess() {
              Ok(status) => state_bar.write().text = status.state_message().into(),
              Err(e) => state_bar.write().text = e.message().into(),
            },
            color: palette.surface_variant(),
            @ { Label::new("Enter") }
          }
        }
      }
    }
  }

  fn chars_grid(&self) -> impl WidgetBuilder {
    let this = self.clone_writer();
    fn_widget! {
      @Column {
        main_axis_gap: 5.,
        align_items: Align::Center,
        justify_content: JustifyContent::Center,
        @ {
          (0..$this.max_rounds()).map(move |row| {
            @Row {
              main_axis_gap: 5.,
              align_items: Align::Center,
              justify_content: JustifyContent::Center,
              @ {
                pipe! {
                  (0..$this.len_hint()).map(move |col| @ { $this.char_grid(row, col) })
                }
              }
            }
          })
        }
      }
    }
  }
}

impl<T: StateWriter<Value = Wordle>> WordleExtraWidgets for T {}

fn hint_color(hint: Option<CharHint>, ctx: &BuildCtx) -> Color {
  let palette = Palette::of(ctx);
  hint.map_or_else(
    || palette.surface_variant(),
    |s| match s {
      CharHint::Correct => palette.success(),
      CharHint::WrongPosition => palette.warning(),
      CharHint::Wrong => palette.error(),
    },
  )
}

impl Wordle {
  fn char_hint(&self, row: usize, col: usize) -> Option<WordleChar> {
    assert!(col < self.len_hint());
    return match row.cmp(&self.guesses.len()) {
      std::cmp::Ordering::Less => Some(*self.guesses[row].char_hint(col)),
      std::cmp::Ordering::Equal if col < self.guessing.word().len() => Some(WordleChar {
        char: self.guessing.word().chars().nth(col).unwrap(),
        hint: None,
      }),
      _ => return None,
    };
  }

  fn char_grid(&self, row: usize, col: usize) -> impl WidgetBuilder {
    let char_hint = self.char_hint(row, col);
    let c = char_hint.map(|c| c.char).unwrap_or('\0');
    let hint = char_hint.and_then(|c| c.hint);

    fn_widget! {
      let color = hint_color(hint, ctx!());
      let palette = Palette::of(ctx!());

      let color = palette.container_of(&color);
      let font_color = palette.on_container_of(&color);
      @Container {
        size: Size::new(56., 56.),
        background: color,
        border_radius: Radius::all(4.),
        @Text {
          text_style: TypographyTheme::of(ctx!()).display_small.text.clone(),
          foreground: font_color,
          h_align: HAlign::Center,
          v_align: VAlign::Center,
          text: c.to_string(),
        }
      }
    }
  }
}

impl Compose for Wordle {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      let state_bar = @Text { text: "" };
      let keyboard = this.keyboard(state_bar.clone_writer());

      let give_up = @OutlinedButton {
        on_tap: move |_| {
          let status = $this.write().give_up();
          $state_bar.write().text = status.state_message().into();
        },
        @ { Label::new("Give up") }
      };
      let new_game = @FilledButton {
        on_tap: move |_| {
          $this.write().reset();
          $state_bar.write().text = "".into();
        },
        @ { Label::new("New game") }
      };

      @Container {
        size: Size::new(f32::INFINITY, f32::INFINITY),
        auto_focus: true,
        on_chars: move |e| {
          e.chars.chars().for_each(|c| $this.write().guessing.enter_char(c))
        },
        on_key_down: move |e| {
          match e.key {
            VirtualKey::Named(NamedKey::Backspace) => $this.write().guessing.delete_back_char(),
            VirtualKey::Named(NamedKey::Enter) => {
              match $this.write().guess() {
                Ok(status) => $state_bar.write().text = status.state_message().into(),
                Err(e) => $state_bar.write().text = e.message().into(),
              }
            },
            _ => {}
          }
        },
        @Column {
          margin: EdgeInsets::only_top(10.),
          h_align: HAlign::Center,
          align_items: Align::Center,
          justify_content: JustifyContent::Center,
          main_axis_gap: 5.,
          @H1 { text: "Wordle" }
          @Divider { extent: 20. }
          @ {this.chars_grid()}
          @ { state_bar }
          @ { keyboard }
          @Row {
            margin: EdgeInsets::only_top(10.),
            main_axis_gap: 15.,
            @ { give_up }
            @ { new_game }
          }
        }
      }
    }
  }
}
