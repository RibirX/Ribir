use crate::{prelude::*, ticker::FrameMsg};

#[derive(Clone, Copy, PartialEq)]
pub enum HAnchor {
  /// Anchor the left edge position of the widget
  Left(f32),

  /// Anchor the right edge position of the widget
  Right(f32),
}

#[derive(Clone, Copy, PartialEq)]
pub enum VAnchor {
  /// Anchor the top edge position of the widget
  Top(f32),

  /// Anchor the bottom edge position of the widget
  Bottom(f32),
}

#[derive(Clone, Copy, Default)]
pub struct AnchorPosition {
  x: Option<HAnchor>,
  y: Option<VAnchor>,
}

impl AnchorPosition {
  pub fn new(x: HAnchor, y: VAnchor) -> Self { Self { x: Some(x), y: Some(y) } }

  pub fn left(x: f32) -> Self { Self { x: Some(HAnchor::Left(x)), y: None } }

  pub fn right(x: f32) -> Self { Self { x: Some(HAnchor::Right(x)), y: None } }

  pub fn top(y: f32) -> Self { Self { x: None, y: Some(VAnchor::Top(y)) } }

  pub fn bottom(y: f32) -> Self { Self { x: None, y: Some(VAnchor::Bottom(y)) } }

  pub fn top_left(x: f32, y: f32) -> Self { Self::new(HAnchor::Left(x), VAnchor::Top(y)) }

  fn to_offset(self, size: &Size) -> (Option<f32>, Option<f32>) {
    let x = self.x.map(|x| match x {
      HAnchor::Left(x) => x,
      HAnchor::Right(x) => x - size.width,
    });
    let y = self.y.map(|y| match y {
      VAnchor::Top(y) => y,
      VAnchor::Bottom(y) => y - size.height,
    });
    (x, y)
  }
}

#[derive(Declare, Query)]
pub struct GlobalAnchor {
  #[declare(builtin, default)]
  global_anchor: AnchorPosition,
}

impl ComposeChild for GlobalAnchor {
  type Child = Widget;
  #[inline]
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn eq(f1: f32, f2: f32) -> bool { (f1 - f2).abs() < f32::EPSILON }
    fn_widget! {
      let wnd = ctx!().window();
      let tick_of_layout_ready = wnd
        .frame_tick_stream()
        .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));

      let mut child = @$child {};
      let wid = child.lazy_id();
      watch!(($this.get_global_anchor(), $child.layout_size()))
        .sample(tick_of_layout_ready)
        .subscribe(move |_| {
          let size = $child.layout_size();
          let base = wnd.map_to_global(Point::zero(), wid.wid_uncheck());
          let (x, y) = $this.get_global_anchor().to_offset(&size);
          let x = x.map_or(0., |x| x - base.x);
          let y = y.map_or(0., |y| y - base.y);
          if !eq($child.left_anchor, x) {
            $child.write().left_anchor = x;
          }
          if !eq($child.top_anchor, y) {
            $child.write().top_anchor = y;
          }
        });
      child
        .widget_build(ctx!())
        .attach_state_data(this, ctx!())
    }
  }
}

impl GlobalAnchor {
  fn get_global_anchor(&self) -> AnchorPosition { self.global_anchor }
}

impl<W> FatObj<W> {
  /// Anchor the widget's horizontal position to the left edge of the widget
  /// with WidgetId wid .
  pub fn relative_to_left(
    &mut self,
    relative: HAnchor,
    wid: &LazyWidgetId,
    ctx: &BuildCtx,
  ) -> impl Subscription {
    let this = self.get_builtin_global_anchor(ctx).clone_writer();
    let wnd = ctx.window();
    let wid = wid.clone();
    let tick_of_layout_ready = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    tick_of_layout_ready.subscribe(move |_| {
      let base = wnd.map_to_global(Point::zero(), wid.wid_uncheck()).x;
      let anchor = relative.offset(base);
      if this.read().global_anchor.x != Some(anchor) {
        this.write().global_anchor.x = Some(anchor);
      }
    })
  }

  /// Anchor the widget's horizontal position to the right edge of the widget
  /// with WidgetId wid .
  pub fn relative_to_right(
    &mut self,
    relative: HAnchor,
    wid: &LazyWidgetId,
    ctx: &BuildCtx,
  ) -> impl Subscription {
    let this = self.get_builtin_global_anchor(ctx).clone_writer();
    let wnd = ctx.window();
    let wid = wid.clone();
    let tick_of_layout_ready = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    tick_of_layout_ready.subscribe(move |_| {
      let base = wnd.map_to_global(Point::zero(), wid.wid_uncheck()).x;
      let size = wnd.layout_size(wid.wid_uncheck()).unwrap_or_default();
      let anchor = relative.offset(base).offset(size.width);
      if this.read().global_anchor.x != Some(anchor) {
        this.write().global_anchor.x = Some(anchor);
      }
    })
  }

  /// Anchor the widget's vertical position to the top edge of the widget
  /// with WidgetId wid.
  pub fn relative_to_top(
    &mut self,
    relative: VAnchor,
    wid: &LazyWidgetId,
    ctx: &BuildCtx,
  ) -> impl Subscription {
    let this = self.get_builtin_global_anchor(ctx).clone_writer();
    let wnd = ctx.window();
    let wid = wid.clone();
    let tick_of_layout_ready = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    tick_of_layout_ready.subscribe(move |_| {
      let base = wnd.map_to_global(Point::zero(), wid.wid_uncheck()).y;
      let anchor = relative.offset(base);
      if this.read().global_anchor.y != Some(anchor) {
        this.write().global_anchor.y = Some(anchor);
      }
    })
  }

  /// Anchor the widget's vertical position to the bottom edge of the widget
  /// with WidgetId wid.
  pub fn relative_to_bottom(
    &mut self,
    relative: VAnchor,
    wid: &LazyWidgetId,
    ctx: &BuildCtx,
  ) -> impl Subscription {
    let this = self.get_builtin_global_anchor(ctx).clone_writer();
    let wnd = ctx.window();
    let wid = wid.clone();
    let tick_of_layout_ready = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    tick_of_layout_ready.subscribe(move |_| {
      let base = wnd.map_to_global(Point::zero(), wid.wid_uncheck()).y;
      let size = wnd.layout_size(wid.wid_uncheck()).unwrap_or_default();
      let anchor = relative.offset(base).offset(size.height);
      if this.read().global_anchor.y != Some(anchor) {
        this.write().global_anchor.y = Some(anchor);
      }
    })
  }
}

impl HAnchor {
  pub fn offset(self, offset: f32) -> Self {
    match self {
      HAnchor::Left(x) => HAnchor::Left(x + offset),
      HAnchor::Right(x) => HAnchor::Right(x + offset),
    }
  }
}

impl VAnchor {
  pub fn offset(self, offset: f32) -> Self {
    match self {
      VAnchor::Top(x) => VAnchor::Top(x + offset),
      VAnchor::Bottom(x) => VAnchor::Bottom(x + offset),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn global_anchor() {
    reset_test_env!();
    let (base_offset, offset_writer) = split_value(10.);
    let (wid, wid_writer) = split_value(None);

    let w = fn_widget! {
      let child1 = @MockBox {
        left_anchor: 10.,
        top_anchor: 10.,
        size: Size::new(10., 10.),
      };
      let child1_id = child1.lazy_host_id();
      let mut follow_widget = @MockBox {
        size: Size::new(10., 10.)
      };
      *wid_writer.write() = Some(follow_widget.lazy_host_id());
      follow_widget.relative_to_left(HAnchor::Left(0.), &child1_id, ctx!());
      follow_widget.relative_to_bottom(VAnchor::Bottom(20.), &child1_id, ctx!());

      @MockBox {
        left_anchor: pipe!(*$base_offset),
        size: Size::new(100., 100.),
        @MockStack {
          child_pos: vec![
              Point::new(0., 10.),
              Point::new(10., 0.),
            ],
          @ { child1 }
          @ { follow_widget }
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();

    let follow_wid = wid.read().clone().unwrap();
    assert_eq!(
      wnd.map_to_global(Point::zero(), follow_wid.wid_uncheck()),
      Point::new(20., 40.)
    );

    *offset_writer.write() = 20.;
    wnd.draw_frame();
    assert_eq!(
      wnd.map_to_global(Point::zero(), follow_wid.wid_uncheck()),
      Point::new(30., 40.)
    );
  }
}
