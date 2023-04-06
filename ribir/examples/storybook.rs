use ribir::prelude::{svgs, *};

const WINDOW_SIZE: f32 = 800.;

fn main() {
  let widgets = widget! {
    init ctx => {
      let title_style = TypographyTheme::of(ctx).display_large.text.clone();
      let foreground = Palette::of(ctx).on_surface_variant().into();
      let secondary: Brush = Palette::of(ctx).secondary().into();
      let tertiary: Brush = Palette::of(ctx).tertiary().into();
    }
    ConstrainedBox {
      clamp: BoxClamp::fixed_size(Size::splat(WINDOW_SIZE)),
      Tabs {
        pos: Position::Bottom,
        Tab {
          TabItem {
            svgs::HOME
            Label::new("Button")
          }
          TabPane {
            Column {
              margin: EdgeInsets::all(20.),
              item_gap: 20.,
              Text::new("Button", &foreground, title_style.clone())
              Row {
                item_gap: 20.,
                FilledButton { svgs::ADD }
                FilledButton { Label::new("Filled button") }
                FilledButton {
                  color: secondary.clone(),
                  svgs::ADD
                  Label::new("Filled button")
                }
                FilledButton {
                  color: tertiary.clone(),
                  svgs::ADD
                  Label::new("Filled button")
                }
              }
              Row {
                item_gap: 20.,
                OutlinedButton { svgs::ADD }
                OutlinedButton { Label::new("Outlined button") }
                OutlinedButton {
                  color: secondary.clone(),
                  svgs::ADD
                  Label::new("Outlined button")
                }
                OutlinedButton {
                  color: tertiary.clone(),
                  svgs::ADD
                  Label::new("Outlined button")
                }
              }
              Row {
                item_gap: 20.,
                Button { svgs::ADD }
                Button { Label::new("Raw button") }
                Button {
                  color: secondary.clone(),
                  Label::new("Raw button")
                }
                Button {
                  color: tertiary.clone(),
                  Label::new("Raw button")
                }
              }
              Row {
                item_gap: 20.,
                FabButton { svgs::ADD }
                FabButton { Label::new("Fab button") }
                FabButton {
                  color: secondary,
                  svgs::ADD
                  Label::new("Fab button")
                }
                FabButton {
                  color: tertiary,
                  svgs::ADD
                  Label::new("Fab button")
                }
              }
            }
          }
        }
        Tab {
          TabItem {
            svgs::HOME
            Label::new("Lists")
          }
          TabPane {
            Column {
              margin: EdgeInsets::all(20.),
              Text::new("Lists", &foreground, title_style.clone())
              Lists {
                margin: EdgeInsets::only_top(20.),
                ListItem {
                  line_number: 1,
                  Leading { svgs::CHECK_BOX_OUTLINE_BLANK }
                  HeadlineText(Label::new("One line list item"))
                  SupportingText(Label::new("One line supporting text"))
                }
                Divider { indent: 16. }
                ListItem {
                  Leading { svgs::MENU }
                  HeadlineText(Label::new("One line list item"))
                  Trailing { Label::new("100+") }
                }
                Divider { indent: 16. }
                ListItem {
                  line_number: 2,
                  Leading {
                    Avatar {
                      ShallowImage::from_png(include_bytes!("../../gpu/examples/leaves.png"))
                    }
                  }
                  HeadlineText(Label::new("Two lines list item"))
                  SupportingText(Label::new("Two lines supporting text \rTwo lines supporting text"))
                  Trailing { Label::new("100+") }
                }
                Divider { indent: 16. }
                ListItem {
                  line_number: 1,
                  Leading {
                    ShallowImage::from_png(include_bytes!("../../gpu/examples/leaves.png"))
                  }
                  HeadlineText(Label::new("One lines list item"))
                  SupportingText(Label::new("One lines supporting text"))
                  Trailing { svgs::CHECK_BOX_OUTLINE_BLANK }
                }
                Divider { indent: 16. }
                ListItem {
                  line_number: 1,
                  Leading {
                    Avatar {
                      Label::new("A")
                    }
                  }
                  HeadlineText(Label::new("One lines list item"))
                  SupportingText(Label::new("One lines supporting text"))
                  Trailing { Label::new("100+") }
                }
                Divider { indent: 16. }
                ListItem {
                  line_number: 1,
                  Leading {
                    Poster(ShallowImage::from_png(include_bytes!("../../gpu/examples/leaves.png")))
                  }
                  HeadlineText(Label::new("One lines list item"))
                  SupportingText(Label::new("One lines supporting text"))
                  Trailing { Label::new("100+") }
                }
              }
            }
          }
        }
      }
    }
  };
  let app = Application::new(material::purple::light());
  let wnd = Window::builder(widgets)
    .with_inner_size(Size::new(WINDOW_SIZE, WINDOW_SIZE))
    .with_title("StoryBook")
    .build(&app);
  app::run_with_window(app, wnd);
}
