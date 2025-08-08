use std::cell::Cell;

use crate::prelude::*;
/// Enumerate to describe which direction allow widget to scroll.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Hash)]
pub enum Scrollable {
  /// let child widget horizontal scrollable and the scroll view is as large as
  /// its parent allow.
  X,
  /// Let child widget vertical scrollable and the scroll view is as large  as
  /// its parent allow.
  #[default]
  Y,
  /// Let child widget both scrollable in horizontal and vertical, and the
  /// scroll view is as large as its parent allow.
  Both,
}

/// Helper struct for builtin scrollable field.
#[derive(Default)]
pub struct ScrollableWidget {
  pub scrollable: Scrollable,
  scroll_pos: Point,
  page: Size,
  content_size: Size,

  view_id: Option<TrackId>,
}

impl Declare for ScrollableWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for ScrollableWidget {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut view = @Viewport {
        clip_boundary: true,
        scroll_dir: distinct_pipe!($read(this).scrollable),
        on_wheel: move |e| $write(this).scroll(-e.delta_x, -e.delta_y),
      };

      let mut child = FatObj::new(child);
      let child = @(child) {
        anchor: distinct_pipe!{
          let pos = $read(this).get_scroll_pos();
          Anchor::left_top(-pos.x, -pos.y)
        },
        on_performed_layout: move |e| {
          let content_size = e.box_size().unwrap_or_default();
          let mut this = $write(this);
          if this.content_size != content_size {
            this.set_content_size(content_size);
          }
        }
      };

      this.write().view_id = Some(view.track_id());

      @(view) {
        on_performed_layout: move |_| {
          let view_size = $read(view).size.get();
          let mut this = $write(this);
          if this.page != view_size {
            this.set_page(view_size);
          }
        },
        providers: [Provider::writer(this.clone_writer(), None)],
        @ { child }
      }
    }
    .into_widget()
  }
}

impl ScrollableWidget {
  /// Returns the reference of the closest scrollable widget from the context.
  #[inline]
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> Option<QueryRef<'_, Self>> {
    Provider::of::<Self>(ctx)
  }

  /// Returns the write reference of the closest scrollable widget from the
  /// context.
  #[inline]
  pub fn write_of(ctx: &impl AsRef<ProviderCtx>) -> Option<WriteRef<'_, Self>> {
    Provider::write_of::<Self>(ctx)
  }

  /// Returns the writer of the closest scrollable widget from the context.
  #[inline]
  pub fn writer_of(ctx: &impl AsRef<ProviderCtx>) -> Option<Box<dyn StateWriter<Value = Self>>> {
    Provider::writer_of::<Self>(ctx).map(|s| s.clone_writer())
  }

  pub fn map_to_view(&self, p: Point, child: WidgetId, wnd: &Window) -> Option<Point> {
    let view_id = self.view_id.as_ref()?.get()?;
    let pos = wnd.map_to_global(p, child);
    let base = wnd.map_to_global(Point::zero(), view_id);
    Some(pos - base.to_vector())
  }

  pub fn map_to_content(&self, p: Point, child: WidgetId, wnd: &Window) -> Option<Point> {
    let content_id = self
      .view_id
      .as_ref()?
      .get()?
      .single_child(wnd.tree())?;
    let pos = wnd.map_to_global(p, child);
    let base = wnd.map_to_global(Point::zero(), content_id);
    Some(pos - base.to_vector())
  }

  /// Keep the content box visible in the scroll view with the given anchor
  /// relative to the view.
  ///
  /// If Anchor.x is None,  it will anchor the widget to the closest edge of the
  /// view in horizontal direction, when the widget is out of the view.
  /// If Anchor.y is None, it will anchor the widget to the closest edge of the
  /// view in vertical direction, when the widget is out of the view.
  pub fn visible_content_box(&mut self, rect: Rect, anchor: Anchor) {
    let view_size = self.scroll_view_size();

    let offset_x = anchor
      .x
      .or_else(|| {
        if rect.max_x() > self.scroll_pos.x + view_size.width {
          Some(HAnchor::Right(0.0.into()))
        } else if rect.min_x() < self.scroll_pos.x {
          Some(HAnchor::Left(0.0.into()))
        } else {
          None
        }
      })
      .map_or(self.scroll_pos.x, |x| rect.min_x() - x.into_pixel(rect.width(), view_size.width));

    let offset_y = anchor
      .y
      .or_else(|| {
        if rect.max_y() > view_size.height + self.scroll_pos.y {
          Some(VAnchor::Bottom(0.0.into()))
        } else if rect.min_y() < self.scroll_pos.y {
          Some(VAnchor::Top(0.0.into()))
        } else {
          None
        }
      })
      .map_or(self.scroll_pos.y, |y| rect.min_y() - y.into_pixel(rect.height(), view_size.height));

    self.jump_to(Point::new(offset_x, offset_y));
  }

  /// Ensure the given child is visible in the scroll view with the given anchor
  /// relative to the view.
  /// If Anchor.x is None,  it will anchor the widget to the closest edge of the
  /// view in horizontal direction, when the widget is out of the view.
  /// If Anchor.y is None, it will anchor the widget to the closest edge of the
  /// view in vertical direction, when the widget is out of the view.
  pub fn visible_widget(&mut self, child: WidgetId, anchor: Anchor, wnd: &Window) {
    let Some(pos) = self.map_to_content(Point::zero(), child, wnd) else { return };
    let Some(size) = wnd.widget_size(child) else { return };
    let show_box = Rect::new(pos, size);
    self.visible_content_box(show_box, anchor);
  }

  pub fn scroll(&mut self, x: f32, y: f32) {
    let mut new = self.scroll_pos;
    if self.scrollable != Scrollable::X {
      new.y += y;
    }
    if self.scrollable != Scrollable::Y {
      new.x += x;
    }
    self.jump_to(new);
  }

  pub fn jump_to(&mut self, top_left: Point) {
    let max = self.max_scrollable();
    self.scroll_pos = top_left.clamp(Point::zero(), max.to_vector().to_point());
  }

  #[inline]
  pub fn scroll_view_size(&self) -> Size { self.page }

  #[inline]
  pub fn scroll_content_size(&self) -> Size { self.content_size }

  pub fn is_x_scrollable(&self) -> bool {
    self.scrollable != Scrollable::Y && self.content_size.width > self.page.width
  }

  pub fn is_y_scrollable(&self) -> bool {
    self.scrollable != Scrollable::X && self.content_size.height > self.page.height
  }

  pub fn max_scrollable(&self) -> Point {
    let max = self.scroll_content_size() - self.scroll_view_size();
    max.to_vector().to_point().max(Point::zero())
  }

  /// Return the pixel along the axis of the scrollable widget that you want
  /// displayed in the upper left.
  pub fn get_scroll_pos(&self) -> Point { self.scroll_pos }

  pub fn get_x_scroll_rate(&self) -> f32 {
    let pos = self.scroll_pos.x;
    if pos.is_normal() { pos / self.max_scrollable().x } else { 0. }
  }

  pub fn get_y_scroll_rate(&self) -> f32 {
    let pos = self.scroll_pos.y;
    if pos.is_normal() { pos / self.max_scrollable().y } else { 0. }
  }

  fn sync_pos(&mut self) { self.jump_to(self.scroll_pos) }

  fn set_content_size(&mut self, content_size: Size) {
    self.content_size = content_size;
    self.sync_pos()
  }

  fn set_page(&mut self, page: Size) {
    self.page = page;
    self.sync_pos()
  }
}

#[derive(SingleChild, Declare)]
struct Viewport {
  scroll_dir: Scrollable,
  #[declare(skip)]
  size: Cell<Size>,
}

impl Render for Viewport {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut child_clamp = clamp;
    if self.scroll_dir != Scrollable::X {
      child_clamp.max.height = f32::INFINITY;
    }
    if self.scroll_dir != Scrollable::Y {
      child_clamp.max.width = f32::INFINITY;
    }

    let mut size = ctx.assert_perform_single_child_layout(child_clamp);
    if self.scroll_dir != Scrollable::X && clamp.max.height.is_infinite() {
      size.height = clamp.container_height(size.height);
    }
    if self.scroll_dir != Scrollable::Y && clamp.max.width.is_infinite() {
      size.width = clamp.container_width(size.width);
    }
    size = clamp.clamp(size);
    // The viewport needs to accurately record its real size, as widgets like
    // `padding` may increase the size without the viewport accounting for the
    // additional space.
    self.size.set(size);

    size
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  fn test_assert(scrollable: Scrollable, delta_x: f32, delta_y: f32, expect_x: f32, expect_y: f32) {
    let w = fn_widget! {
      @MockBox {
        size: Size::new(1000., 1000.),
        scrollable,
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    wnd.process_wheel(delta_x, delta_y);

    wnd.draw_frame();
    let pos = wnd.layout_info_by_path(&[0, 0]).unwrap().pos;
    assert_eq!(pos, Point::new(expect_x, expect_y));
  }

  #[test]
  fn x_scroll() {
    reset_test_env!();

    test_assert(Scrollable::X, -10., -10., -10., 0.);
    test_assert(Scrollable::X, -10000., -10., -900., 0.);
    test_assert(Scrollable::X, 100., -10., 0., 0.);
  }

  #[test]
  fn y_scroll() {
    reset_test_env!();

    test_assert(Scrollable::Y, -10., -10., 0., -10.);
    test_assert(Scrollable::Y, -10., -10000., 0., -900.);
    test_assert(Scrollable::Y, 10., 100., 0., 0.);
  }

  #[test]
  fn both_scroll() {
    reset_test_env!();

    test_assert(Scrollable::Both, -10., -10., -10., -10.);
    test_assert(Scrollable::Both, -10000., -10000., -900., -900.);
    test_assert(Scrollable::Both, 100., 100., 0., 0.);
  }

  #[derive(SingleChild, Declare, Clone)]
  pub struct FixedBox {
    pub size: Size,
  }

  impl Render for FixedBox {
    #[inline]
    fn perform_layout(&self, _: BoxClamp, ctx: &mut LayoutCtx) -> Size {
      ctx.perform_single_child_layout(BoxClamp { min: self.size, max: self.size });
      self.size
    }
    #[inline]
    fn size_affected_by_child(&self) -> bool { false }
    #[inline]
    fn paint(&self, _: &mut PaintingCtx) {}
  }

  #[test]
  fn scroll_content_expand() {
    reset_test_env!();

    let w = fn_widget! {
      @FixedBox {
        size: Size::new(200., 200.),
        @ScrollableWidget {
          scrollable: Scrollable::Both,
          on_performed_layout: move |ctx| {
            assert_eq!(ctx.box_size(), Some(Size::new(200., 200.)));
          },
          @MockBox {
            size: Size::new(100., 100.),
            on_performed_layout: move |ctx| {
              assert_eq!(ctx.box_size(), Some(Size::new(200., 200.)));
            },
          }
        }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
  }
}
