use ribir_core::prelude::*;

use crate::layout::{Stack, StackFit};

/// A control widget that enables the user to access horizontal parts child that
/// is larger than the box rect.
#[derive(Declare, Clone)]
pub struct HScrollBar {
  /// Scrolled pixels of child content.
  #[declare(default)]
  pub offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBarStyle {
  /// The min size of the thumb have.
  pub thumb_min_size: f32,
  /// The thickness of scrollbar element.
  pub thickness: f32,
  /// The brush of the scrollbar track.
  pub track_brush: Brush,
}

/// Compose style that use to decoration the thumb of horizontal scrollbar,
/// overwrite it when init theme.
#[derive(Debug, Declare)]
pub struct HScrollBarThumbDecorator {
  pub offset: f32,
}

impl ComposeDecorator for HScrollBarThumbDecorator {
  fn compose_decorator(this: State<Self>, host: Widget) -> impl WidgetBuilder {
    fn_widget! { @$host { anchor: pipe!($this.offset).map(Anchor::left) } }
  }
}

/// Compose style that use to decoration the thumb of vertical scrollbar,
/// overwrite it when init theme.
#[derive(Debug, Declare)]
pub struct VScrollBarThumbDecorator {
  pub offset: f32,
}

impl ComposeDecorator for VScrollBarThumbDecorator {
  fn compose_decorator(this: State<Self>, host: Widget) -> impl WidgetBuilder {
    fn_widget! { @$host { anchor: pipe!($this.offset).map(Anchor::top) } }
  }
}

impl ComposeChild for HScrollBar {
  type Child = Widget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let mut scrolling = @ScrollableWidget {
        scrollable: Scrollable::X,
        scroll_pos: Point::new($this.offset, 0.),
      };
      let scrollbar = @HRawScrollbar {
        scrolling: scrolling.get_scrollable_widget().clone_watcher(),
        v_align: VAlign::Bottom,
      };

      // `scrolling` and `this` have same lifetime, so we needn't unsubscribe.
      watch!($scrolling.scroll_pos.x)
        .distinct_until_changed()
        .subscribe(move |v| $this.write().offset = v);
      let u = watch!($this.offset)
        .distinct_until_changed()
        .subscribe(move |v| {
          let y = $scrolling.scroll_pos.y;
          $scrolling.write().jump_to(Point::new(v, y));
        });

      @Stack {
        fit: StackFit::Passthrough,
        on_disposed: move |_| { u.unsubscribe(); },
        @ $scrolling { @{ child } }
        @ { scrollbar }
      }
    }
  }
}

/// A control widget that enables the user to access vertical parts child that
/// is larger than the box rect.
#[derive(Declare, Clone)]
pub struct VScrollBar {
  /// Scrolled pixels of child content.
  #[declare(default)]
  pub offset: f32,
}

impl ComposeChild for VScrollBar {
  type Child = Widget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let mut scrolling = @ScrollableWidget {
        scrollable: Scrollable::Y,
        scroll_pos: Point::new(0., $this.offset),
      };

      let scrollbar = @VRawScrollbar {
        scrolling: scrolling.get_scrollable_widget().clone_watcher(),
        h_align: HAlign::Right
      };

      // `scrolling` and `this` have same lifetime, so we needn't unsubscribe.
      watch!($scrolling.scroll_pos.y)
        .distinct_until_changed()
        .subscribe(move |v| $this.write().offset = v);
      let u = watch!($this.offset)
        .distinct_until_changed()
        .subscribe(move |v| {
          let x = $scrolling.scroll_pos.x;
          $scrolling.write().jump_to(Point::new(x, v));
        });

      @Stack {
        fit: StackFit::Passthrough,
        on_disposed: move |_| { u.unsubscribe(); },
        @ $scrolling { @{ child } }
        @ { scrollbar }
      }
    }
  }
}
/// A control widget that enables the user to access horizontal parts child that
/// is larger than the box rect.
#[derive(Declare, Clone)]
pub struct BothScrollbar {
  /// Scrolled pixels of child content.
  #[declare(default)]
  pub offset: Point,
}

impl ComposeChild for BothScrollbar {
  type Child = Widget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let mut scrolling = @ScrollableWidget {
        scrollable: Scrollable::Both,
        scroll_pos: $this.offset,
      };
      let mut h_bar = @HRawScrollbar {
        scrolling: scrolling.get_scrollable_widget().clone_watcher(),
        v_align: VAlign::Bottom,
      };
      let mut v_bar = @VRawScrollbar {
        scrolling: scrolling.get_scrollable_widget().clone_watcher(),
        h_align: HAlign::Right,
        margin: EdgeInsets::only_bottom($h_bar.layout_height())
      };

      // `scrolling` and `this` have same lifetime, so we needn't unsubscribe.
      watch!($scrolling.scroll_pos)
        .distinct_until_changed()
        .subscribe(move |v| $this.write().offset = v);
      let u = watch!($this.offset)
        .distinct_until_changed()
        .subscribe(move |v| $scrolling.write().jump_to(v) );

      @Stack{
        fit: StackFit::Passthrough,
        on_disposed: move |_| { u.unsubscribe(); },
        @ $scrolling { @{ child } }
        @ $h_bar{ margin: EdgeInsets::only_right($v_bar.layout_width()) }
        @ { v_bar }
      }
    }
  }
}

/// A widget that display the horizontal scrolling information of the
/// `scrolling` widget.
#[derive(Declare)]
pub struct HRawScrollbar {
  scrolling: Watcher<Reader<ScrollableWidget>>,
}

impl Compose for HRawScrollbar {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      @ {
        let scrolling = $this.scrolling.clone_watcher();
        let ScrollBarStyle {
          thickness,
          thumb_min_size,
          track_brush,
        } = ScrollBarStyle::of(ctx!());

        let mut track_box = @Container {
          size: Size::new(f32::MAX, 0.),
          background: track_brush
        };

        let thumb_outline = @HScrollBarThumbDecorator {
          offset: pipe!{
            let scrolling = $scrolling;
            let content_width = scrolling.scroll_content_size().width;
            -scrolling.scroll_pos.x * safe_recip(content_width) * $track_box.layout_width()
          }
        };

        let mut container = @Container {
          size: {
            let scrolling = $scrolling;
            let page_width = scrolling.scroll_view_size().width;
            let content_width = scrolling.scroll_content_size().width;
            let width = page_width / content_width * $track_box.layout_width();
            Size::new(width.max(thumb_min_size), thickness)
          },
        };

        watch!($container.layout_height())
          .distinct_until_changed()
          .subscribe(move |v| $track_box.write().size.height = v);

        @Stack {
          visible: pipe! {
            let scrolling = $scrolling;
            scrolling.can_scroll()
          },
          @ { track_box }
          @$thumb_outline {
            @ { container }
          }
        }
      }

    }
  }
}

/// A widget that display the vertical scrolling information of the
/// `scrolling` widget.
#[derive(Declare)]
pub struct VRawScrollbar {
  scrolling: Watcher<Reader<ScrollableWidget>>,
}

impl Compose for VRawScrollbar {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      @ {
        let scrolling = $this.scrolling.clone_watcher();
        let ScrollBarStyle {
          thickness,
          thumb_min_size,
          ref track_brush
        } = ScrollBarStyle::of(ctx!());

        let mut track_box = @Container {
          size: Size::new(0., f32::MAX),
          background: track_brush.clone()
        };

        let thumb_outline = @VScrollBarThumbDecorator {
          offset: pipe! {
            let scrolling = $scrolling;
            let content_height = scrolling.scroll_content_size().height;
            -scrolling.scroll_pos.y * safe_recip(content_height) * $track_box.layout_height()
          }
        };

        let mut container = @Container {
          size: pipe! {
            let scrolling = $scrolling;
            let page_height = scrolling.scroll_view_size().height;
            let content_height = scrolling.scroll_content_size().height;
            let height = page_height / content_height * $track_box.layout_height();
            Size::new(thickness, height.max(thumb_min_size))
          },
        };

        watch!($container.layout_width())
          .distinct_until_changed()
          .subscribe(move |v| $track_box.write().size.width = v);

        @Stack {
          visible: pipe! { $scrolling.can_scroll() },
          @ { track_box }
          @$thumb_outline {
            @ { container }
          }
        }
      }
    }
  }
}

fn safe_recip(v: f32) -> f32 {
  let v = v.recip();
  if v.is_infinite() || v.is_nan() { 0. } else { v }
}

impl CustomStyle for ScrollBarStyle {
  fn default_style(ctx: &BuildCtx) -> Self {
    ScrollBarStyle {
      thumb_min_size: 12.,
      thickness: 8.,
      track_brush: Palette::of(ctx).primary_container().into(),
    }
  }
}

#[cfg(test)]
mod test {
  use ribir_core::{reset_test_env, test_helper::*};
  use ribir_dev_helper::*;

  use super::*;
  use crate::layout::{Column, ConstrainedBox};

  fn content_expand_so_all_view_can_scroll() -> impl WidgetBuilder {
    fn_widget! {
      @ConstrainedBox {
        clamp: BoxClamp::EXPAND_BOTH,
        @Stack {
          fit: StackFit::Passthrough,
          @HScrollBar {
            @Container { size: Size::new(100., 100.) }
          }
          @VScrollBar {
            @Container { size: Size::new(100., 100.) }
          }
          @BothScrollbar {
            @Container { size: Size::new(100., 100.) }
          }
        }
      }
    }
  }
  widget_layout_test!(
    content_expand_so_all_view_can_scroll,
    wnd_size = Size::new(200., 200.),
    { path = [0, 0, 0], width == 200., height == 200., }
    { path = [0, 0, 1], width == 200., height == 200., }
    { path = [0, 0, 2], width == 200., height == 200., }
  );

  #[test]
  fn scrollable() {
    reset_test_env!();

    let offset = Stateful::new(Point::zero());
    let v_offset = Stateful::new(0.);
    let h_offset = Stateful::new(0.);
    let c_offset = offset.clone_writer();
    let c_v_offset = v_offset.clone_reader();
    let c_h_offset = h_offset.clone_reader();
    let w = fn_widget! {
      let both_bar = @BothScrollbar { offset: pipe!(*$offset) };
      let h_bar = @HScrollBar { offset: pipe!($both_bar.offset.x) };
      let v_bar = @VScrollBar { offset: pipe!($both_bar.offset.y) };

      watch!($v_bar.offset)
        .subscribe(move|v| *$v_offset.write() = v);
      watch!($h_bar.offset)
        .subscribe(move|v| *$h_offset.write() = v);

      let container_size = Size::new(100., 100.);
      @Column {
        @Container {
          size: Size::new(30., 30.),
          @$both_bar { @Container { size: container_size } }
        }
        @Container {
          size: Size::new(30., 30.),
          @$h_bar { @Container { size: container_size } }
        }
        @Container {
          size: Size::new(30., 30.),
          @$v_bar { @Container { size: container_size } }
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(1024., 1024.));
    wnd.draw_frame();
    {
      *c_offset.write() = Point::new(-10., -10.);
    }
    {
      *c_offset.write() = Point::new(-20., -20.);
    }
    wnd.draw_frame();
    assert_eq!(*c_v_offset.read(), c_offset.read().y);
    assert_eq!(*c_h_offset.read(), c_offset.read().x);
  }
}
