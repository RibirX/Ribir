use crate::prelude::*;

/// A control widget that enables the user to access horizontal parts child that
/// is larger than the box rect.
#[derive(Declare, Clone)]
pub struct HScrollBar {
  /// Scrolled pixels of child content.
  #[declare(default)]
  pub offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBarTheme {
  /// The min size of the thumb have.
  pub thumb_min_size: f32,
  /// The thickness of scrollbar element.
  pub thickness: f32,
  /// The margin of scrollbar thumb at cross axis edge in logical pixels.
  pub thumb_margin: f32,
}

impl ComposeChild for HScrollBar {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
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
          scrolling: scrolling.clone_stateful(),
          v_align: VAlign::Bottom,
        }
      }
      change_on scrolling.pos.x ~> this.offset
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
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
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
          scrolling: scrolling.clone_stateful(),
          h_align: HAlign::Right
        }
      }

      change_on scrolling.pos.y ~> this.offset
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
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      env {
        let theme = ScrollBarTheme::custom_theme_of(ctx.theme());
        let margin = theme.track_thickness();
      }
      Stack {
        ScrollableWidget {
          id: scrolling,
          scrollable: Scrollable::Both,
          pos: this.offset,
          ExprWidget { expr: child}
        }
        HRawScrollbar {
          scrolling: scrolling.clone_stateful(),
          v_align: VAlign::Bottom,
          margin: EdgeInsets::only_right(margin)
        }
        VRawScrollbar {
          scrolling: scrolling.clone_stateful(),
          h_align: HAlign::Right,
          margin: EdgeInsets::only_bottom(margin)
        }
      }
      change_on scrolling.pos ~> this.offset
    }
  }
}

/// A widget that display the horizontal scrolling information of the
/// `scrolling` widget.
#[derive(Declare)]
pub struct HRawScrollbar {
  scrolling: Stateful<ScrollableWidget>,
}

impl Compose for HRawScrollbar {
  fn compose(this: StateWidget<Self>) -> Widget {
    let this = this.into_stateful();
    let scrolling = this.raw_ref().scrolling.clone();

    widget! {
      track { scrolling, this }
      env {
        let theme = ScrollBarTheme::custom_theme_of(ctx.theme());
      }
      LayoutBox {
        id: track_box,
        SizedBox {
          size: Size::new(f32::MAX, theme.track_thickness()),
          compose_styles: [cs::SCROLLBAR_TRACK],
          SizedBox {
            id: thumb,
            size: {
              let page_width = scrolling.page_size().width;
              let content_width = scrolling.content_size().width;
              let width = page_width / content_width * track_box.width();
              Size::new(width.max(theme.thumb_min_size), theme.thickness)
            },
            left_anchor: {
              let content_width = scrolling.content_size().width;
              -scrolling.pos.x * safe_recip(content_width) * track_box.width()
            },
            compose_styles: [cs::SCROLLBAR_THUMB],
          }
        }
      }
      change_on thumb.left_anchor Animate {
        transition: transitions::SMOOTH_SCROLL.get_from_or_default(ctx),
        lerp_fn: move |from, to, rate| {
          let from = from.abs_value(thumb.size.width);
          let to = to.abs_value(thumb.size.width);
          PositionUnit::Pixel(from.lerp(&to, rate))
        }
      }
    }
  }
}

/// A widget that display the vertical scrolling information of the
/// `scrolling` widget.
#[derive(Declare)]
pub struct VRawScrollbar {
  scrolling: Stateful<ScrollableWidget>,
}

impl Compose for VRawScrollbar {
  fn compose(this: StateWidget<Self>) -> Widget {
    let this = this.into_stateful();
    let scrolling = this.raw_ref().scrolling.clone();

    widget! {
      track { scrolling, this }
      env {
        let theme = ScrollBarTheme::custom_theme_of(ctx.theme());
      }
      LayoutBox {
        id: track_box,
        SizedBox {
          size: Size::new(theme.track_thickness(), f32::MAX),
          compose_styles: [cs::SCROLLBAR_TRACK],
          SizedBox {
            id: thumb,
            size: {
              let page_height = scrolling.page_size().height;
              let content_height = scrolling.content_size().height;
              let height = page_height / content_height * track_box.height();
              Size::new( theme.thickness, height.max(theme.thumb_min_size))
            },
            top_anchor: {
              let content_height = scrolling.content_size().height;
              -scrolling.pos.y * safe_recip(content_height) * track_box.height()
            },
            compose_styles: [cs::SCROLLBAR_THUMB],
          }
        }
      }
      change_on thumb.top_anchor Animate {
        transition: transitions::SMOOTH_SCROLL.get_from_or_default(ctx),
        lerp_fn: move |from, to, rate| {
          let from = from.abs_value(thumb.size.height);
          let to = to.abs_value(thumb.size.height);
          PositionUnit::Pixel(from.lerp(&to, rate))
        }
      }
    }
  }
}

fn safe_recip(v: f32) -> f32 {
  let v = v.recip();
  if v.is_infinite() || v.is_nan() { 0. } else { v }
}

impl CustomTheme for ScrollBarTheme {}

impl Default for ScrollBarTheme {
  fn default() -> Self {
    Self {
      thumb_min_size: 12.,
      thickness: 8.,
      thumb_margin: 2.,
    }
  }
}

impl ScrollBarTheme {
  fn track_thickness(&self) -> f32 { self.thickness + self.thumb_margin * 2. }
}
