use material::material_svgs;
use ribir::prelude::*;

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

pub fn messages() -> impl Into<Widget> {
  MessageList {
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
  }
}

impl Compose for MessageList {
  fn compose(this: State<Self>) -> Widget {
    fn_widget! {
      let palette = Palette::of(ctx!());

      @Column {
        background: palette.surface(),
        @Row {
          justify_content: JustifyContent::SpaceBetween,
          padding: EdgeInsets::new(8., 16., 8., 16.),
          align_items: Align::Center,
          @Row {
            item_gap: 10.,
            @TinyIcon { @{ svgs::MENU } }
            @Text {
              text: "Message",
              foreground: palette.on_surface(),
              text_style: TypographyTheme::of(ctx!()).title_large.text.clone(),
            }
          }
          @Row {
            item_gap: 10.,
            @TinyIcon { @{ svgs::SEARCH } }
            @TinyIcon { @{ svgs::MORE_VERT } }
          }
        }
        @Tabs {
          pos: Position::Bottom,
          @Tab {
            @TabItem {
              @{ material_svgs::SMS }
              @{ Label::new("Messages") }
            }
            @TabPane {
              @VScrollBar {
                @Lists {
                  @{
                    let message_gen = |message: Message| {
                      @Column {
                        @ListItem {
                          line_number: 1usize,
                          @HeadlineText(Label::new(message.nick_name.clone()))
                          @SupportingText(Label::new(message.content.clone()))
                          @Leading {
                            @Avatar {
                              @{
                                let name = message.avatar.to_string();
                                let mut avatar = format!("{}/examples/attachments/3DDD-{name}.png", env!("CARGO_WORKSPACE_DIR"));
                                let img = PixelImage::from_png(&std::fs::read(avatar).unwrap());
                                ShareResource::new(img)
                              }
                            }
                          }
                          @Trailing { @{ svgs::MORE_HORIZ } }
                        }
                        @Divider {}
                      }
                    };

                    Multi::new($this.messages.clone().into_iter().map(message_gen))
                  }
                }
              }
            }
          }
          @Tab {
            @TabItem {
              @{ material_svgs::ACCOUNT_CIRCLE }
              @{ Label::new("Person") }
            }
            @TabPane { @Text { text: "Person" } }
          }
        }
      }
    }
    .into()
  }
}
