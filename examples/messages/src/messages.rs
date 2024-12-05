use material::material_svgs;
use ribir::prelude::*;

#[derive(Clone)]
struct Message {
  nick_name: String,
  content: String,
  img: Resource<PixelImage>,
}

#[derive(Clone)]
struct MessageList {
  messages: Vec<Message>,
}

pub fn messages() -> Widget<'static> {
  MessageList {
    messages: vec![
      Message {
        nick_name: "James Harden".to_string(),
        content: "Coming soon!".to_string(),
        img: Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-2.png"))),
      },
      Message {
        nick_name: "Allen Iverson".to_string(),
        content: "You are welcome!".to_string(),
        img: Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-1.png"))),
      },
      Message {
        nick_name: "Kyrie Irving".to_string(),
        content: "See you next week!".to_string(),
        img: Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-3.png"))),
      },
      Message {
        nick_name: "Jaylon Lee".to_string(),
        content: "Fighting!".to_string(),
        img: Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-4.png"))),
      },
    ],
  }
  .into_widget()
}

impl Compose for MessageList {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let palette = Palette::of(BuildCtx::get());
      @Column {
        text_line_height: 24.,
        background: palette.surface(),
        @Row {
          justify_content: JustifyContent::SpaceBetween,
          padding: EdgeInsets::new(8., 16., 8., 16.),
          align_items: Align::Center,
          @Row {
            item_gap: 10.,
            @Icon { @{ svgs::MENU } }
            @Text {
              text: "Message",
              foreground: palette.on_surface(),
              text_style: TypographyTheme::of(BuildCtx::get()).title_large.text.clone(),
            }
          }
          @Row {
            item_gap: 10.,
            @Icon { @{ svgs::SEARCH } }
            @Icon { @{ svgs::MORE_VERT } }
          }
        }
        @Tabs {
          pos: Position::Bottom,
          @Tab {
            @TabItem {
              @{ material_svgs::SMS }
              @{ Label::new("Messages") }
            }
            @TabPane(
              fn_widget! {
                @Scrollbar {
                  @Lists {
                    @{
                      let message_gen = move |message: Message| {
                        @Column {
                          @ListItem {
                            line_number: 1usize,
                            @HeadlineText(Label::new(message.nick_name.clone()))
                            @SupportingText(Label::new(message.content.clone()))
                            @Leading(
                              EdgeWidget::Avatar(@Avatar { @{ message.img.clone() } })
                            )
                            @Trailing(EdgeWidget::Icon(svgs::MORE_HORIZ.into_widget()))
                          }
                          @Divider {}
                        }
                      };

                      $this.messages.clone().into_iter().map(message_gen)
                    }
                  }
                }
              }.into()
            )
          }
          @Tab {
            @TabItem {
              @{ material_svgs::ACCOUNT_CIRCLE }
              @{ Label::new("Person") }
            }
            @TabPane(fn_widget!(@Text { text: "Person" }).into())
          }
        }
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material};
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    messages,
    WidgetTester::new(messages)
      .with_wnd_size(Size::new(400., 600.))
      .with_comparison(0.004)
  );
}
