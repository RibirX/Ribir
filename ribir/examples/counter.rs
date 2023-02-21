use ribir::prelude::*;

fn main() {
  let w = widget! {
    init ctx => {
      let ease_in = transitions::EASE_IN.of(ctx);
      let headline3_style = TypographyTheme::of(ctx).headline3.text.clone();
    }
    states {
      cnt: Stateful::new(0_i32),
    }
    Column {
      Row {
        margin: EdgeInsets::all(2.),
        Button {
          on_tap: move |_| *cnt += 1,
          margin: EdgeInsets::only_right(2.),
          ButtonText::new("Add")
        }
        Button {
          on_tap: move |_| *cnt -= 1,
          margin: EdgeInsets::only_right(2.),
          ButtonText::new("Sub")
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
          style: headline3_style,
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
  app::run(w);
}
