use ribir::prelude::*;

use crate::wordle::{CharHint, Wordle, WordleChar};

pub fn wordle_game() -> Widget<'static> { Wordle::new(5, 5).into_widget() }

impl Wordle {
  fn chars_key<const N: usize>(
    this: &impl StateWriter<Value = Wordle>, chars: [char; N],
  ) -> impl Iterator<Item = Widget<'static>> + 'static {
    chars.into_iter().map({
      let this = this.clone_writer();
      move |c| Wordle::char_key(&this, c)
    })
  }

  fn char_key(this: &impl StateWriter<Value = Wordle>, key: char) -> Widget<'static> {
    let palette = Palette::of(BuildCtx::get());
    let base = palette.base_of(&palette.surface_variant());
    let success = palette.success();
    let warning = palette.warning();
    let error = palette.error();
    let (color, u) = Stateful::from_pipe(pipe! {
      $read(this).key_hint(key).map_or(
        base,
        |s| match s {
          CharHint::Correct => success,
          CharHint::WrongPosition => warning,
          CharHint::Wrong => error,
        })
    });

    filled_button! {
      providers: [Provider::writer(color, None)],
      on_tap: move |_| $write(this).guessing.enter_char(key),
      on_disposed:  move |_| u.unsubscribe(),
      @ { key.to_string() }
    }
    .into_widget()
  }

  fn keyboard(
    this: impl StateWriter<Value = Wordle>, state_bar: Stateful<Text>,
  ) -> Widget<'static> {
    let palette = Palette::of(BuildCtx::get());
    let gray = palette.base_of(&palette.surface_variant());
    flex! {
      y: AnchorY::center(),
      direction: Direction::Vertical,
      item_gap: 5.,
      align_items: Align::Center,
      justify_content: JustifyContent::Start,
      @Flex {
        x: AnchorX::center(),
        item_gap: 5.,
        align_items: Align::Center,
        justify_content: JustifyContent::Start,
        @Wordle::chars_key(&this, ['Q', 'W', 'E', 'R','T', 'Y', 'U', 'I','O', 'P'])
      }
      @Flex {
        x: AnchorX::center(),
        item_gap: 5.,
        align_items: Align::Center,
        justify_content: JustifyContent::Start,
        @Wordle::chars_key(&this, ['A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L' ])
      }
      @Flex {
        x: AnchorX::center(),
        item_gap: 5.,
        align_items: Align::Center,
        justify_content: JustifyContent::Start,
        @FilledButton {
          providers: [Provider::new(gray)],
          on_tap: move |_| $write(this).guessing.delete_back_char(),
          @ { "Del" }
        }
        @Wordle::chars_key(&this, ['Z', 'X', 'C', 'V', 'B', 'N', 'M' ])

        @FilledButton {
          providers: [Provider::new(gray)],
          on_tap: move |_| match $write(this).guess() {
            Ok(status) => state_bar.write().text = status.state_message().into(),
            Err(e) => state_bar.write().text = e.message().into(),
          },
          @ { "Enter" }
        }
      }
    }
    .into_widget()
  }

  fn chars_grid(this: &impl StateWriter<Value = Wordle>) -> Widget<'static> {
    fn_widget! {
      @Flex {
        y: AnchorY::center(),
        direction: Direction::Vertical,
        item_gap: 5.,
        align_items: Align::Center,
        justify_content: JustifyContent::Start,
        @ {
          (0..$read(this).max_rounds()).map(move |row| {
            @Flex {
              y: AnchorY::center(),
              item_gap: 5.,
              align_items: Align::Center,
              justify_content: JustifyContent::Start,
              @pipe! {
                (0..$read(this).len_hint())
                  .map(move |col| fn_widget! { $read(this).char_grid(row, col) })
              }
            }
          })
        }
      }
    }
    .into_widget()
  }
}

fn hint_color(hint: Option<CharHint>) -> Color {
  let palette = Palette::of(BuildCtx::get());
  hint.map_or_else(
    || palette.base_of(&palette.surface_variant()),
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
    match row.cmp(&self.guesses.len()) {
      std::cmp::Ordering::Less => Some(*self.guesses[row].char_hint(col)),
      std::cmp::Ordering::Equal if col < self.guessing.word().len() => {
        Some(WordleChar { char: self.guessing.word().chars().nth(col).unwrap(), hint: None })
      }
      _ => None,
    }
  }

  fn char_grid(&self, row: usize, col: usize) -> Widget<'static> {
    let char_hint = self.char_hint(row, col);
    let c = char_hint.map(|c| c.char).unwrap_or('\0');
    let hint = char_hint.and_then(|c| c.hint);

    fn_widget! {
      let color = hint_color(hint);
      let palette = Palette::of(BuildCtx::get());

      let color = palette.container_of(&color);
      let font_color = palette.on_container_of(&color);
      @Container {
        width: 56.,
        height: 56.,
        background: color,
        radius: Radius::all(4.),
        @Text {
          text_style: TypographyTheme::of(BuildCtx::get()).display_small.text.clone(),
          foreground: font_color,
          x: AnchorX::center(),
          y: AnchorY::center(),
          text: c.to_string(),
        }
      }
    }
    .into_widget()
  }
}

impl Compose for Wordle {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let state_bar = @Text { text: "" };
      let keyboard = Wordle::keyboard($writer(this), state_bar.clone_writer());

      let give_up = @Button {
        on_tap: move |_| {
          let status = $write(this).give_up();
          $write(state_bar).text = status.state_message().into();
        },
        @ { "Give up" }
      };
      let new_game = @FilledButton {
        on_tap: move |_| {
          $write(this).reset();
          $write(state_bar).text = "".into();
        },
        @ { "New game" }
      };

      @Container {
        auto_focus: true,
        on_chars: move |e| {
          e.chars.chars().for_each(|c| $write(this).guessing.enter_char(c))
        },
        on_key_down: move |e| {
          match e.key() {
            VirtualKey::Named(NamedKey::Backspace) => $write(this).guessing.delete_back_char(),
            VirtualKey::Named(NamedKey::Enter) => {
              match $write(this).guess() {
                Ok(status) => $write(state_bar).text = status.state_message().into(),
                Err(e) => $write(state_bar).text = e.message().into(),
              }
            },
            _ => {}
          }
        },
        @Flex {
          clamp: BoxClamp::EXPAND_BOTH,
          direction: Direction::Vertical,
          margin: EdgeInsets::only_top(10.),
          x: AnchorX::center(),
          y: AnchorY::center(),
          align_items: Align::Center,
          justify_content: JustifyContent::Start,
          item_gap: 5.,
          @H1 { text: "Wordle" }
          @Divider { }
          @Wordle::chars_grid(&this)
          @ { state_bar }
          @ { keyboard }
          @Flex {
            margin: EdgeInsets::only_top(10.),
            item_gap: 15.,
            @ { give_up }
            @ { new_game }
          }
        }
      }
    }
    .into_widget()
  }
}
