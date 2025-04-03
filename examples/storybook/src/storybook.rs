use ribir::{material::material_svgs, prelude::*};
use webbrowser::{Browser, open_browser};

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
    scrollbar! {
      @Stack {
        @Column {
          h_align: HAlign::Center,
          align_items: Align::Center,
          @Column {
            align_items: Align::Center,
            @Row {
              clamp: BoxClamp::fixed_height(30.),
              @Text { text: "Common buttons" }
              @Icon { @ { material_svgs::INFO } }
            }
            @Column {
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_low(),
              radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Row {
                item_gap: 20.,
                @FilledButton { @ {"Filled" } }
                @FilledButton {
                  @Icon { @{ svgs::SETTINGS } }
                  @ { "Icon" }
                }
              }
              @Row {
                item_gap: 20.,
                @Button { @ { "Outlined" } }
                @Button {
                  @Icon { @Icon { @ { svgs::SEARCH } } }
                  @ { "Icon" }
                }
              }
              @Row {
                item_gap: 20.,
                @TextButton { @ { "Text" } }
                @TextButton {
                  @Icon { @ { svgs::ADD } }
                  @ { "Icon" }
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
                @Icon { @ { material_svgs::INFO } }
              }
            }
            @Column {
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_lowest(),
              radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Row {
                item_gap: 20.,
                @Fab { @Icon { @ { svgs::ADD } } }
                @Fab {
                  @Icon { @ { svgs::ADD } }
                  @ { "Create" }
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
                @Icon { @ { material_svgs::INFO } }
              }
            }
            @Column {
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_lowest(),
              radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Row {
                item_gap: 20.,
                @TextButton { @Icon { @ { svgs::SETTINGS } } }
                @FilledButton { @Icon { @ { svgs::SETTINGS } } }
                @Button { @Icon{ @ { svgs::SETTINGS } } }
              }
            }
          }
        }
      }
    }
    .into()
  }

  fn tabs_show() -> GenWidget {
    fn_widget! {
      @Tabs {
        @Tab {
          @ { "Videos" }
          @Icon { @ { svgs::HOME } }
          @void! {}
        }
        @Tab {
          @ { "Photos" }
          @Icon { @ { svgs::HOME } }
          @void! {}
        }
        @Tab {
          @ { "Audio" }
          @Icon { @ { svgs::HOME } }
          @void! {}
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
        @List {
          margin: EdgeInsets::only_top(20.),
          @ListItem {
            interactive: true,
            on_tap: move |_| {
              if let Err(err) = open_browser(Browser::Default, "https://ribir.org") {
                println!("Failed to open browser: {}", err);
              }
            },
            @Icon { @{ svgs::CHECK_BOX_OUTLINE_BLANK } }
            @ListItemHeadline { @ { "One line list item" } }
            @ListItemSupporting { @ { "One line supporting text"}}
          }
          @Divider { indent: DividerIndent::Start }
          @ListItem {
            @Icon { @ { svgs::MENU } }
            @ListItemHeadline { @{ "One line list item"} }
            @ListItemTrailingSupporting { @{ "100+" } }
          }
          @Divider { indent: DividerIndent::Start }
          @ListItem {
            @Avatar {
              @Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-1.png")))
            }
            @ListItemHeadline { @ { "Two lines list item" }}
            @ListItemSupporting {
              lines: 2usize,
              @ { "Two lines supporting text \rTwo lines supporting text" }
            }
            @Trailing { @Icon {  @ { svgs::CHECK_BOX_OUTLINE_BLANK } } }
          }
          @Divider { indent: DividerIndent::Start }
          @ListItem {
            @Avatar { @ { "A" } }
            @ListItemHeadline{ @ { "One lines list item" } }
            @ListItemSupporting{ @ { "One lines supporting text" }}
            @ListItemTrailingSupporting { @ { "100+" } }
          }
          @Divider { indent: DividerIndent::Start }
          @ListItem {
            @ListItemThumbNail {
              @Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-3.png")))
            }
            @ListItemHeadline { @ { "One lines list item" } }
            @ListItemSupporting { @ { "One lines supporting text" } }
            @ListItemTrailingSupporting { @ { "100+" } }
          }
        }
      }
    }
    .into()
  }

  fn checkbox_show() -> GenWidget {
    self::column! {
      margin: EdgeInsets::all(20.),
      @List {
        @ListItem {
          @Checkbox { }
          @ListItemHeadline { @{ "Option1"  } }
        }
        @ListItem {
          @Checkbox { }
          @ListItemHeadline { @{ "Option2"  } }
        }
        @ListItem {
          @Checkbox { }
          @ListItemHeadline { @{ "Option3"  } }
        }
      }
    }
    .into()
  }

  fn_widget! {
    @Tabs {
      h_align: HAlign::Stretch,
      providers: [Provider::new(TabPos::Bottom)],
      @Tab {
        @ { "Actions" }
        @actions_show()
      }
      @Tab {
        @ { "Tabs" }
        @tabs_show()
      }
      @Tab {
        @ { "Containment" }
        @containment_show()
      }
      @Tab {
        @ { "Lists" }
        @lists_show()
      }
      @Tab {
        @ { "Selections" }
        @checkbox_show()
      }
    }
  }
  .into_widget()
}

pub fn storybook() -> Widget<'static> {
  fn_widget! {
    @Column {
      align_items: Align::Center,
      background: Palette::of(BuildCtx::get()).surface_container_low(),
      @ { header() }
      @Expanded {
        @ { content() }
      }
    }
  }
  .into_widget()
}
