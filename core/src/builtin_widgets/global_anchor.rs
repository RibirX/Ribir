use std::rc::Rc;

use crate::{prelude::*, ticker::FrameMsg};

#[derive(Query, Default)]
pub struct GlobalAnchor {
  pub global_anchor: Anchor,
}

impl Declare for GlobalAnchor {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl ComposeChild for GlobalAnchor {
  type Child = Widget;
  #[inline]
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let wnd = ctx!().window();
      let tick_of_layout_ready = wnd
        .frame_tick_stream()
        .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));

      let mut child = @$child {};
      let wid = child.lazy_id();
      let u = watch!(($this.get_global_anchor(), $child.layout_size()))
        .sample(tick_of_layout_ready)
        .subscribe(move |(_, size)| {
          let wnd_size = wnd.size();
          let base = wnd.map_to_global(Point::zero(), wid.assert_id());
          // The global anchor may change during sampling, so we need to retrieve it again.
          let Anchor {x, y} = $this.get_global_anchor();
          let anchor = Anchor {
            x: x.map(|x| match x {
              HAnchor::Left(x) => x - base.x,
              HAnchor::Right(x) => (wnd_size.width - x) - size.width - base.x,
            }).map(HAnchor::Left),
            y: y.map(|y| match y {
              VAnchor::Top(y) => y - base.y,
              VAnchor::Bottom(y) => (wnd_size.height - y) - size.height - base.y,
            }).map(VAnchor::Top),
          };
          if $child.anchor != anchor {
            $child.write().anchor = anchor;
          }
        });

      @ $child { on_disposed: move |_| { u.unsubscribe(); } }
    }
  }
}

impl GlobalAnchor {
  fn get_global_anchor(&self) -> Anchor { self.global_anchor }
}

fn bind_h_anchor(
  this: &impl StateWriter<Value = GlobalAnchor>, wnd: &Rc<Window>, relative: HAnchor, base: f32,
) {
  let size = wnd.size();
  let anchor = match relative {
    HAnchor::Left(x) => HAnchor::Left(base + x),
    HAnchor::Right(x) => HAnchor::Right((size.width - base) + x),
  };
  if this.read().global_anchor.x != Some(anchor) {
    this.write().global_anchor.x = Some(anchor);
  }
}

fn bind_v_anchor(
  this: &impl StateWriter<Value = GlobalAnchor>, wnd: &Rc<Window>, relative: VAnchor, base: f32,
) {
  let size = wnd.size();
  let anchor = match relative {
    VAnchor::Top(x) => VAnchor::Top(base + x),
    VAnchor::Bottom(x) => VAnchor::Bottom((size.height - base) + x),
  };
  if this.read().global_anchor.y != Some(anchor) {
    this.write().global_anchor.y = Some(anchor);
  }
}

impl<W> FatObj<W> {
  /// Anchor the widget's horizontal position by placing its left edge right to
  /// the left edge of the specified widget (`wid`) with the given relative
  /// pixel value (`relative`).
  pub fn left_align_to(
    &mut self, wid: &LazyWidgetId, offset: f32, ctx: &BuildCtx,
  ) -> impl Subscription {
    let this = self.get_global_anchor_widget().clone_writer();
    let wnd = ctx.window();
    let wid = wid.clone();
    let tick_of_layout_ready = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    tick_of_layout_ready.subscribe(move |_| {
      let base = wnd
        .map_to_global(Point::zero(), wid.assert_id())
        .x;
      bind_h_anchor(&this, &wnd, HAnchor::Left(offset), base);
    })
  }

  /// Anchor the widget's horizontal position by placing its right edge left to
  /// the right edge of the specified widget (`wid`) with the given relative
  /// pixel value (`relative`).
  pub fn right_align_to(
    &mut self, wid: &LazyWidgetId, relative: f32, ctx: &BuildCtx,
  ) -> impl Subscription {
    let this = self.get_global_anchor_widget().clone_writer();
    let wnd = ctx.window();
    let wid = wid.clone();
    let tick_of_layout_ready = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    tick_of_layout_ready.subscribe(move |_| {
      let base = wnd
        .map_to_global(Point::zero(), wid.assert_id())
        .x;
      let size = wnd
        .layout_size(wid.assert_id())
        .unwrap_or_default();
      bind_h_anchor(&this, &wnd, HAnchor::Right(relative), base + size.width);
    })
  }

  /// Anchors the widget's vertical position by placing its top edge below the
  /// top edge of the specified widget (`wid`) with the given relative pixel
  /// value (`relative`).
  pub fn top_align_to(
    &mut self, wid: &LazyWidgetId, relative: f32, ctx: &BuildCtx,
  ) -> impl Subscription {
    let this = self.get_global_anchor_widget().clone_writer();
    let wnd = ctx.window();
    let wid = wid.clone();
    let tick_of_layout_ready = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    tick_of_layout_ready.subscribe(move |_| {
      let base = wnd
        .map_to_global(Point::zero(), wid.assert_id())
        .y;
      bind_v_anchor(&this, &wnd, VAnchor::Top(relative), base);
    })
  }

  /// Anchors the widget's vertical position by placing its bottom edge above
  /// the bottom edge of the specified widget (`wid`) with the given relative
  /// pixel value (`relative`).
  pub fn bottom_align_to(
    &mut self, wid: &LazyWidgetId, relative: f32, ctx: &BuildCtx,
  ) -> impl Subscription {
    let this = self.get_global_anchor_widget().clone_writer();
    let wnd = ctx.window();
    let wid = wid.clone();
    let tick_of_layout_ready = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    tick_of_layout_ready.subscribe(move |_| {
      let base = wnd
        .map_to_global(Point::zero(), wid.assert_id())
        .y;
      let size = wnd
        .layout_size(wid.assert_id())
        .unwrap_or_default();
      bind_v_anchor(&this, &wnd, VAnchor::Bottom(relative), base + size.height);
    })
  }
}
#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  const WND_SIZE: Size = Size::new(100., 100.);
  fn global_anchor() -> impl WidgetBuilder {
    fn_widget! {
      let parent = @MockBox {
        anchor: Anchor::left_top(10., 10.),
        size: Size::new(50., 50.),
      };
      let wid = parent.lazy_host_id();
      let mut top_left = @MockBox {
        size: Size::new(10., 10.),
      };
      top_left.left_align_to(&wid, 20., ctx!());
      top_left.top_align_to(&wid, 10., ctx!());

      let mut bottom_right = @MockBox {
        size: Size::new(10., 10.),
      };
      bottom_right.right_align_to(&wid, 10.,  ctx!());
      bottom_right.bottom_align_to(&wid, 20., ctx!());
      @ $parent {
        @MockStack {
          child_pos: vec![Point::new(0., 0.), Point::new(0., 0.)],
          @ { top_left }
          @ { bottom_right }
        }
      }
    }
  }

  widget_layout_test!(
    global_anchor,
    wnd_size = WND_SIZE,
    { path = [0, 0, 0, 0, 0], x == 20.,}
    { path = [0, 0, 0, 0, 0], y == 10.,}

    { path = [0, 0, 0, 1, 0], x == 30.,}
    { path = [0, 0, 0, 1, 0], y == 20.,}
  );
}
