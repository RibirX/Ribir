#[cfg(any(feature = "crossterm", feature = "winit"))]
fn main() {
  use ribir::prelude::*;

  let w = widget! {
    init ctx => {
      let ease_in = transitions::EASE_IN.of(ctx);
      let style = TypographyTheme::of(ctx).display_medium.text.clone();
    }
    states {
      cnt: Stateful::new(0_i32),
    }
    Column {
      Row {
        margin: EdgeInsets::all(2.),
        FilledButton {
          on_tap: move |_| *cnt += 1,
          margin: EdgeInsets::only_right(2.),
          Label::new("Add")
        }
        FilledButton {
          on_tap: move |_| *cnt -= 1,
          margin: EdgeInsets::only_right(2.),
          Label::new("Sub")
        }
      }
      Row {
        Text { text: "current count:" }
        Text {
          id: text,
          text: {
            let cnt = *cnt;
            format!("{cnt}")
          },
          style,
        }
      }
    }
    Animate {
      id: animate,
      transition: ease_in,
      prop: prop!(text.transform),
      from: Transform::translation(0., text.layout_height() * -2.)
    }
    finally {
      let_watch!(*cnt)
        .subscribe(move |_| animate.run());
    }
  };

  let mut app = Application::new(material::purple::light());
  let window_builder = app.window_builder(w, Default::default());

  let window_id = app.build_window(window_builder);
  app.exec(window_id);
}

#[cfg(not(any(feature = "crossterm", feature = "winit")))]
fn main() {
  println!("Chose a platform to run:");
  println!("  cargo run --example counter -F winit,wgpu_gl");
  println!("  cargo run --example counter -F crossterm");
}
