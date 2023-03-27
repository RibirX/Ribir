use ribir::prelude::{svgs, *};

const WINDOW_SIZE: f32 = 800.;

fn main() {
  let widgets = widget! {
    init ctx => {
      let title_style = TypographyTheme::of(ctx).display_large.text.clone();
      let foreground = Palette::of(ctx).on_surface_variant().into();
      let primary: Brush = Palette::of(ctx).primary().into();
      let on_primary: Brush = Palette::of(ctx).on_primary().into();
    }
    VScrollBar {

      Column {
        margin: EdgeInsets::horizontal(20.),
        Column {
          margin: EdgeInsets::only_top(20.),
          Text::new("Button", &foreground, title_style.clone())
          Row {
            margin: EdgeInsets::only_top(20.),
            FilledButton { svgs::ADD }
            SizedBox { size: Size::new(20., 0.) }
            FilledButton { Label::new("Filled button") }
            SizedBox { size: Size::new(20., 0.) }
            FilledButton {
              color: Color::RED,
              svgs::ADD
              Label::new("Filled button")
            }
            SizedBox { size: Size::new(20., 0.) }
            FilledButton {
              color: Color::GREEN,
              svgs::ADD
              Label::new("Filled button")
            }
          }
          Row {
            margin: EdgeInsets::only_top(20.),
            OutlinedButton { svgs::ADD }
            SizedBox { size: Size::new(20., 0.) }
            OutlinedButton { Label::new("Outlined button") }
            SizedBox { size: Size::new(20., 0.) }
            OutlinedButton {
              color: Color::RED,
              svgs::ADD
              Label::new("Outlined button")
            }
            SizedBox { size: Size::new(20., 0.) }
            OutlinedButton {
              color: Color::GREEN,
              svgs::ADD
              Label::new("Outlined button")
            }
          }
          Row {
            margin: EdgeInsets::only_top(20.),
            Button { svgs::ADD }
            SizedBox { size: Size::new(20., 0.) }
            Button { Label::new("Raw button") }
            SizedBox { size: Size::new(20., 0.) }
            Button {
              color: Color::RED,
              Label::new("Raw button")
            }
            SizedBox { size: Size::new(20., 0.) }
            Button {
              color: Color::GREEN,
              Label::new("Raw button")
            }
          }
          Row {
            margin: EdgeInsets::only_top(20.),
            FabButton { svgs::ADD }
            SizedBox { size: Size::new(20., 0.) }
            FabButton { Label::new("Fab button") }
            SizedBox { size: Size::new(20., 0.) }
            FabButton {
              color: Color::RED,
              svgs::ADD
              Label::new("Fab button")
            }
            SizedBox { size: Size::new(20., 0.) }
            FabButton {
              color: Color::GREEN,
              svgs::ADD
              Label::new("Fab button")
            }
          }
        }
        Divider { extent: 30., end_indent: 150. }
        Column {
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
                IntoWidget::into_widget(
                  widget! {
                    Container {
                      size: Size::splat(40.),
                      background: primary.clone(),
                      border_radius: Radius::all(20.),
                      Text {
                        h_align: HAlign::Center,
                        v_align: VAlign::Center,
                        foreground: on_primary.clone(),
                        text: "A",
                      }
                    }
                  }
                )
              }
              HeadlineText(Label::new("Two lines list item"))
              SupportingText(Label::new("Two lines supporting text \rTwo lines supporting text"))
              Trailing { Label::new("100+") }
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
