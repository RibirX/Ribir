use crate::prelude::*;

/// A control widget that enables the user to access horizontal parts child that
/// is larger than the box rect.
#[derive(Declare, Clone)]
pub struct HScrollBar {
  /// Scrolled pixels of child content.
  #[declare(default)]
  pub offset: f32,
  #[declare(default=ScrollBarTheme::of(ctx).clone())]
  pub style: ScrollBarTheme,
}

impl ComposeSingleChild for HScrollBar {
  fn compose_single_child(this: StateWidget<Self>, child: Widget) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      Stack {
        ScrollableWidget {
          id: scrolling,
          scrollable: Scrollable::X,
          pos: Point::new(this.offset, 0.),
          ExprWidget { expr: child}
        }
        HRawScrollbar {
          style: this.style.clone(),
          scrolling: scrolling.clone_stateful(),
          v_align: VAlign::Bottom,
        }
      }
      dataflows {
        #[skip_nc]
        scrolling.pos.x ~> this.offset
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
  #[declare(default=ScrollBarTheme::of(ctx).clone())]
  pub style: ScrollBarTheme,
}

impl ComposeSingleChild for VScrollBar {
  fn compose_single_child(this: StateWidget<Self>, child: Widget) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      Stack {
        ScrollableWidget {
          id: scrolling,
          scrollable: Scrollable::Y,
          pos: Point::new(0., this.offset),
          ExprWidget { expr: child}
        }
        VRawScrollbar {
          style: this.style.clone(),
          scrolling: scrolling.clone_stateful(),
          h_align: HAlign::Right
        }
      }
      dataflows {
        #[skip_nc]
        scrolling.pos.y ~> this.offset
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
  #[declare(default=ScrollBarTheme::of(ctx).clone())]
  pub style: ScrollBarTheme,
}

impl ComposeSingleChild for BothScrollbar {
  fn compose_single_child(this: StateWidget<Self>, child: Widget) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      Stack {
        ScrollableWidget {
          id: scrolling,
          scrollable: Scrollable::Both,
          pos: this.offset,
          ExprWidget { expr: child}
        }
        HRawScrollbar {
          style: this.style.clone(),
          scrolling: scrolling.clone_stateful(),
          v_align: VAlign::Bottom,
          margin: EdgeInsets::only_right(this.style.track.thickness )
        }
        VRawScrollbar {
          style: this.style.clone(),
          scrolling: scrolling.clone_stateful(),
          h_align: HAlign::Right,
          margin: EdgeInsets::only_bottom(this.style.track.thickness )
        }
      }
      dataflows {
        #[skip_nc]
        scrolling.pos ~> this.offset
      }
    }
  }
}

/// A widget that display the horizontal scrolling information of the
/// `scrolling` widget.
#[derive(Declare)]
pub struct HRawScrollbar {
  #[declare(default=ScrollBarTheme::of(ctx).clone())]
  pub style: ScrollBarTheme,
  scrolling: Stateful<ScrollableWidget>,
}

impl Compose for HRawScrollbar {
  fn compose(this: StateWidget<Self>) -> Widget {
    let this = this.into_stateful();
    let scrolling = this.raw_ref().scrolling.clone();
    widget! {
      track { scrolling, this }
      Stack {
        LayoutBox {
          id: track_box,
          SizedBox {
            size: Size::new(f32::MAX, this.style.track.thickness),
            background: this.style.track.background.clone(),
            radius: this.style.track.radius,
          }
        }
        SizedBox {
          size: {
            let page_width = scrolling.page_size().width;
            let content_width = scrolling.content_size().width;
            let width = page_width / content_width * track_box.width();
            Size::new(width.max(this.style.thumb_min_size), this.style.thumb.thickness)
          },
          background: this.style.thumb.background.clone(),
          radius: this.style.thumb.radius,
          left_anchor: {
            let content_width = scrolling.content_size().width;
             -scrolling.pos.x / content_width * track_box.width()
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
  #[declare(default=ScrollBarTheme::of(ctx).clone())]
  pub style: ScrollBarTheme,
  scrolling: Stateful<ScrollableWidget>,
}

impl Compose for VRawScrollbar {
  fn compose(this: StateWidget<Self>) -> Widget {
    let this = this.into_stateful();
    let scrolling = this.raw_ref().scrolling.clone();
    widget! {
      track { scrolling, this }
      Stack {
        LayoutBox {
          id: track_box,
          SizedBox {
            size: Size::new(this.style.track.thickness, f32::MAX),
            background: this.style.track.background.clone(),
            radius: this.style.track.radius,
          }
        }
        SizedBox {
          size: {
            let page_height = scrolling.page_size().height;
            let content_height = scrolling.content_size().height;
            let height = page_height / content_height * track_box.height();
            Size::new( this.style.thumb.thickness, height.max(this.style.thumb_min_size))
          },
          background: this.style.thumb.background.clone(),
          radius: this.style.thumb.radius,
          top_anchor: {
            let content_height = scrolling.content_size().height;
            -scrolling.pos.y / content_height * track_box.height()
          }
        }
      }
    }
  }
}
