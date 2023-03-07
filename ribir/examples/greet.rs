//! This is just a `widget!` syntax teaching demo No consideration to its
//! completeness and reasonableness.

use ribir::prelude::*;

fn main() {
  let hi = widget! {
    states { counter: Stateful::new(0) }
    init ctx => {
      let style = TypographyTheme::of(ctx).display_large.text.clone();
    }
    Column {
      Row {
        align_items: Align::Center,
        Input {
          id: input,
          Placeholder::new("Enter the name you want to greet.")
        }
        FilledButton {
          on_tap: move |_| *counter += 1,
          Label::new({
            let counter = counter.to_string();
            format!("Greet!({counter})")
          })
        }
      }
      DynWidget {
        dyns := assign_watch!(*counter > 0)
          .stream_map(|o| o.distinct_until_changed())
          .map(move |need_greet| {
            let style = style.clone();
            need_greet.then(move || {
              widget! {
                init ctx => {
                  let ease_in = transitions::EASE_IN.of(ctx);
                }
                Row {
                  Text { text: "Hello ", style: style.clone() }
                  Text {
                    id: greet,
                    text: no_watch!(input.text()),
                    style: style.clone()
                  }
                  Text { text: "!" , style }
                }
                Animate {
                  id: greet_new,
                  transition: ease_in,
                  prop: prop!(greet.transform),
                  from: Transform::translation(0., greet.layout_height() * 2.)
                }
                finally {
                  let_watch!(*counter)
                    .subscribe(move |_| {
                      greet.text = input.text();
                      input.set_text("");
                    });
                  let_watch!(greet.text.clone())
                    .subscribe(move |_| greet_new.run());
                }
              }
            })
        })
      }
    }
  };

  app::run(hi);
}
