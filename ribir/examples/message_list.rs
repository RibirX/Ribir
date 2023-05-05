use ribir::prelude::*;
use ribir_theme_material::material_svgs;

#[derive(Clone)]
struct Message {
  avatar: i32,
  nick_name: String,
  content: String,
}

#[derive(Clone)]
struct MessageList {
  messages: Vec<Message>,
}

impl Compose for MessageList {
  type Target = Widget;
  fn compose(this: State<Self>) -> Self::Target {
    widget! {
      states { this: this.into_readonly() }
      init ctx => {
        let title_style = TypographyTheme::of(ctx).title_large.text.clone();
        let title_icon_size = IconSize::of(ctx).tiny;
        let background = Palette::of(ctx).surface();
        let foreground = Palette::of(ctx).on_surface().into();
      }
      Column {
        background,
        Row {
          justify_content: JustifyContent::SpaceBetween,
          padding: EdgeInsets::new(8., 16., 8., 16.),
          align_items: Align::Center,
          Row {
            item_gap: 10.,
            Icon {
              size: title_icon_size,
              svgs::MENU
            }
            Text {
              text: "Message",
              foreground,
              style: title_style.clone(),
            }
          }
          Row {
            item_gap: 10.,
            Icon {
              size: title_icon_size,
              svgs::SEARCH
            }
            Icon {
              size: title_icon_size,
              svgs::MORE_VERT
            }
          }
        }
        Tabs {
          pos: Position::Bottom,
          Tab {
            TabItem {
              material_svgs::SMS
              Label::new("Messages")
            }
            TabPane {
              VScrollBar {
                Lists {
                  DynWidget {
                    dyns: this.messages.clone().into_iter().map(move |message| {
                      let mut avatar = "./ribir/examples/attachments/3DDD-".to_string();
                      avatar.push_str(&message.avatar.to_string());
                      avatar.push_str(".png");
                      let img = ShallowImage::from_png(&std::fs::read(avatar).unwrap());

                      widget! {
                        Column {
                          ListItem {
                            line_number: 1,
                            HeadlineText(Label::new(message.nick_name.clone()))
                            SupportingText(Label::new(message.content.clone()))
                            Leading { Avatar { widget::from(img) } }
                            Trailing { svgs::MORE_HORIZ }
                          }
                          Divider {}
                        }
                      }
                    }),
                  }
                }
              }
            }
          }
          Tab {
            TabItem {
              material_svgs::ACCOUNT_CIRCLE
              Label::new("Person")
            }
            TabPane {
              Text {
                text: "Person"
              }
            }
          }
        }
      }
    }
    .into_widget()
  }
}

fn main() {
  env_logger::init();

  let message_list = MessageList {
    messages: vec![
      Message {
        avatar: 2,
        nick_name: "James Harden".to_string(),
        content: "Coming soon!".to_string(),
      },
      Message {
        avatar: 1,
        nick_name: "Allen Iverson".to_string(),
        content: "You are welcome!".to_string(),
      },
      Message {
        avatar: 3,
        nick_name: "Kyrie Irving".to_string(),
        content: "See you next week!".to_string(),
      },
      Message {
        avatar: 4,
        nick_name: "Jaylon Lee".to_string(),
        content: "Fighting!".to_string(),
      },
    ],
  };

  let theme = ribir_theme_material::purple::dark();
  let app = Application::new(theme);
  let wnd = Window::builder(message_list.into_widget())
    .with_inner_size(Size::new(320., 568.))
    .with_title("Message")
    .build(&app);
  app::run_with_window(app, wnd);
}
