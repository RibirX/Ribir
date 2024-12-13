use ribir::{material::material_svgs, prelude::*};

static NORMAL_BUTTON_SIZE: Size = Size::new(120., 40.);

fn header() -> Widget<'static> {
  static TITLE: &str = "Material Design";
  fn_widget! {
    @Text {
      margin: EdgeInsets::vertical(22.),
      text: TITLE
    }
  }
  .into_widget()
}

fn content() -> Widget<'static> {
  fn actions_show() -> GenWidget {
    fn_widget! {
      @Scrollbar {
        text_line_height: 24.,
        @Column {
          clamp: BoxClamp::EXPAND_X,
          align_items: Align::Center,
          @Column {
            align_items: Align::Center,
            @Row {
              clamp: BoxClamp::fixed_height(30.),
              @Text { text: "Common buttons" }
              @Icon {
                @ { material_svgs::INFO }
              }
            }
            @Column {
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_low(),
              border_radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Row {
                item_gap: 20.,
                @SizedBox {
                  size: NORMAL_BUTTON_SIZE,
                  @FilledButton {
                    @ { Label::new("Filled") }
                  }
                }
                @SizedBox {
                  size: NORMAL_BUTTON_SIZE,
                  @FilledButton {
                    @ { svgs::ADD }
                    @ { Label::new("Icon") }
                  }
                }
              }
              @Row {
                item_gap: 20.,
                @SizedBox {
                  size: NORMAL_BUTTON_SIZE,
                  @OutlinedButton {
                    @ { Label::new("Outlined") }
                  }
                }
                @SizedBox {
                  size: NORMAL_BUTTON_SIZE,
                  @OutlinedButton {
                    @ { svgs::ADD }
                    @ { Label::new("Icon") }
                  }
                }
              }
              @Row {
                item_gap: 20.,
                @SizedBox {
                  size: NORMAL_BUTTON_SIZE,
                  @Button {
                    @ { Label::new("Text") }
                  }
                }
                @SizedBox {
                  size: NORMAL_BUTTON_SIZE,
                  @Button {
                    @ { svgs::ADD }
                    @ { Label::new("Icon") }
                  }
                }
              }
            }
          }
          @Column {
            align_items: Align::Center,
            @ConstrainedBox {
              clamp: BoxClamp::fixed_height(30.),
              @Row {
                @Text { text: "Floating action buttons" }
                @Icon {
                  @ { material_svgs::INFO }
                }
              }
            }
            @Column {
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_lowest(),
              border_radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Row {
                item_gap: 20.,
                @FabButton {
                  @ { svgs::ADD }
                }
                @FabButton {
                  @ { svgs::ADD }
                  @ { Label::new("Create") }
                }
              }
            }
          }
          @Column {
            align_items: Align::Center,
            @ConstrainedBox {
              clamp: BoxClamp::fixed_height(30.),
              @Row {
                @Text { text: "Icon buttons" }
                @Icon {
                  @ { material_svgs::INFO }
                }
              }
            }
            @Column {
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_lowest(),
              border_radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Row {
                item_gap: 20.,
                @Button {
                  @ { svgs::SETTINGS }
                }
                @FilledButton {
                  @ { svgs::SETTINGS }
                }
                @OutlinedButton {
                  @ { svgs::SETTINGS }
                }
              }
            }
          }
        }
      }.into_widget()
    }
    .into()
  }

  fn tabs_show() -> GenWidget {
    fn_widget! {
      @Tabs {
        @Tab {
          @TabItem {
            @ { svgs::HOME }
            @ { Label::new("Video") }
          }
          @TabPane(fn_widget!(Void).into())
        }
        @Tab {
          @TabItem {
            @ { svgs::HOME }
            @ { Label::new("Photos") }
          }
          @TabPane(fn_widget!(Void).into())
        }
        @Tab {
          @TabItem {
            @ { svgs::HOME }
            @ { Label::new("Audio") }
          }
          @TabPane(fn_widget!(Void).into())
        }
      }
    }
    .into()
  }

  fn containment_show() -> GenWidget {
    fn_widget! {
      @Column {
        @ConstrainedBox {
          clamp: BoxClamp::fixed_height(30.),
          @Row {
            h_align: HAlign::Center,
            @Text { text: "Divider" }
            @Icon {
              @ { material_svgs::INFO }
            }
          }
        }
        @Divider {}
      }
    }
    .into()
  }

  fn lists_show() -> GenWidget {
    fn_widget! {
      @Column {
        margin: EdgeInsets::all(20.),
        @Lists {
          margin: EdgeInsets::only_top(20.),
          @UrlLink {
            url: "https://ribir.org",
            @ListItem {
              @Leading::new(EdgeWidget::Icon(svgs::CHECK_BOX_OUTLINE_BLANK.into_widget()))
              @ { HeadlineText(Label::new("One line list item")) }
              @ { SupportingText(Label::new("One line supporting text")) }
            }
          }
          @Divider { indent: 16. }
          @ListItem {
            @Leading::new(EdgeWidget::Icon(svgs::MENU.into_widget()))
            @ { HeadlineText(Label::new("One line list item")) }
            @Trailing::new(EdgeWidget::Text(Label::new("100+")))
          }
          @Divider { indent: 16. }
          @ListItem {
            line_number: 2usize,
            @Leading::new(
              EdgeWidget::Avatar(
                @Avatar {
                  @ { Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-1.png"))) }
                }
              )
            )
            @ { HeadlineText(Label::new("Two lines list item")) }
            @ { SupportingText(Label::new("Two lines supporting text \rTwo lines supporting text")) }
            @Trailing::new(EdgeWidget::Icon(svgs::CHECK_BOX_OUTLINE_BLANK.into_widget()))
          }
          @Divider { indent: 16. }
          @ListItem {
            @Leading::new(EdgeWidget::Avatar(@Avatar { @Label::new("A") }))
            @ { HeadlineText(Label::new("One lines list item")) }
            @ { SupportingText(Label::new("One lines supporting text")) }
            @Trailing::new(EdgeWidget::Text(Label::new("100+")))
          }
          @Divider { indent: 16. }
          @ListItem {
            @Leading::new(EdgeWidget::Poster(
              Poster(Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-3.png"))))
            ))
            @ { HeadlineText(Label::new("One lines list item")) }
            @ { SupportingText(Label::new("One lines supporting text")) }
            @Trailing::new(@EdgeWidget::Text(Label::new("100+")))
          }
        }
      }
    }
    .into()
  }

  fn checkbox_show() -> GenWidget {
    self::column! {
      margin: EdgeInsets::all(20.),
      @Lists {
        @Checkbox { @ { "Option1" } }
        @Checkbox { @ { "Option2" } }
        @Checkbox { @ { "Option3" } }
      }
    }
    .into()
  }

  fn_widget! {
    @Tabs {
      pos: Position::Bottom,
      @Tab {
        @TabItem {
          @ { Label::new("Actions") }
        }
        @TabPane(actions_show())
      }
      @Tab {
        @TabItem {
          @ { Label::new("Tabs") }
        }
        @TabPane(tabs_show())
      }
      @Tab {
        @TabItem {
          @ { Label::new("Containment") }
        }
        @TabPane(containment_show())
      }
      @Tab {
        @TabItem {
          @ { Label::new("Lists") }
        }
        @TabPane(lists_show())
      }
      @Tab {
        @TabItem {
          @ { Label::new("Selections") }
        }
        @TabPane(checkbox_show())
      }
    }
  }
  .into_widget()
}

pub fn storybook() -> Widget<'static> {
  let f = fn_widget! {
    @Column {
      clamp: BoxClamp::EXPAND_X,
      align_items: Align::Center,
      background: Palette::of(BuildCtx::get()).surface_container_low(),
      @ { header() }
      @Expanded {
        @ { content() }
      }
    }
  };
  f()
}
