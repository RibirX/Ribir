use ribir::prelude::*;
use webbrowser::{Browser, open_browser};

fn header() -> Widget<'static> {
  text! {
    margin: EdgeInsets::vertical(22.),
    text: "Material Design"
  }
  .into_widget()
}

fn content() -> Widget<'static> {
  fn actions_show() -> GenWidget {
    scrollbar! {
      h_align: HAlign::Stretch,
      @Stack {
        h_align: HAlign::Center,
        @Column {
          align_items: Align::Center,
          @Column {
            align_items: Align::Center,
            @Row {
              clamp: BoxClamp::fixed_height(30.),
              @Text { text: "Common buttons" }
              @Icon { @ { svg_registry::get_or_default("info") } }
            }
            @Flex {
              direction: Direction::Vertical,
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_low(),
              radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Flex {
                item_gap: 20.,
                @FilledButton { @ {"Filled" } }
                @FilledButton {
                  @Icon { @{ svg_registry::get_or_default("settings") } }
                  @ { "Icon" }
                }
              }
              @Flex {
                item_gap: 20.,
                @Button { @ { "Outlined" } }
                @Button {
                  @Icon { @Icon { @ { svg_registry::get_or_default("search") } } }
                  @ { "Icon" }
                }
              }
              @Flex {
                item_gap: 20.,
                @TextButton { @ { "Text" } }
                @TextButton {
                  @Icon { @ { svg_registry::get_or_default("add") } }
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
                @Icon { @ { svg_registry::get_or_default("info") } }
              }
            }
            @Flex {
              direction: Direction::Vertical,
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_lowest(),
              radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Flex {
                item_gap: 20.,
                @Fab { @Icon { @ { svg_registry::get_or_default("add") } } }
                @Fab {
                  @Icon { @ { svg_registry::get_or_default("add") } }
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
                @Icon { @ { svg_registry::get_or_default("info") } }
              }
            }
            @Flex {
              direction: Direction::Vertical,
              item_gap: 20.,
              padding: EdgeInsets::new(20., 40., 20., 40.),
              background: Palette::of(BuildCtx::get()).surface_container_lowest(),
              radius: Radius::all(4.),
              border: Border::all(BorderSide {
                color: Palette::of(BuildCtx::get()).primary().into(),
                width: 1.,
              }),
              @Flex {
                item_gap: 20.,
                @TextButton { @Icon { @ { svg_registry::get_or_default("settings") } } }
                @FilledButton { @Icon { @ { svg_registry::get_or_default("settings") } } }
                @Button { @Icon{ @ { svg_registry::get_or_default("settings") } } }
              }
            }
          }
        }
      }
    }
    .r_into()
  }

  fn tabs_show() -> GenWidget {
    fn_widget! {
      @Tabs {
        @Tab {
          @ { "Videos" }
          @Icon { @ { svg_registry::get_or_default("home") } }
          @void! {}
        }
        @Tab {
          @ { "Photos" }
          @Icon { @ { svg_registry::get_or_default("home") } }
          @void! {}
        }
        @Tab {
          @ { "Audio" }
          @Icon { @ { svg_registry::get_or_default("home") } }
          @void! {}
        }
      }
    }
    .r_into()
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
              @ { svg_registry::get_or_default("info") }
            }
          }
        }
        @Divider {}
      }
    }
    .r_into()
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
            @Icon { @{ svg_registry::get_or_default("check") } }
            @ListItemHeadline { @ { "One line list item" } }
            @ListItemSupporting { @ { "One line supporting text"}}
          }
          @Divider { indent: DividerIndent::Start }
          @ListItem {
            @Icon { @ { svg_registry::get_or_default("menu") } }
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
            @Trailing { @Icon {  @ { svg_registry::get_or_default("check") } } }
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
            @ListItemThumbnail {
              @Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-3.png")))
            }
            @ListItemHeadline { @ { "One lines list item" } }
            @ListItemSupporting { @ { "One lines supporting text" } }
            @ListItemTrailingSupporting { @ { "100+" } }
          }
        }
      }
    }
    .r_into()
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
    .r_into()
  }

  fn_widget! {
    @Tabs {
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
  flex! {
    direction: Direction::Vertical,
    align_items: Align::Center,
    background: Palette::of(BuildCtx::get()).surface_container_low(),
    @header()
    @Expanded { @ { content() } }
  }
  .into_widget()
}
