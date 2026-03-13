use crate::{prelude::*, wrap_render::*};

/// Allows hidden descendants to continue painting while an outer wrapper keeps
/// a leave animation alive.
#[derive(Clone, Copy, Default)]
pub struct AllowHiddenPaint(pub bool);

/// A wrapper that controls whether its child is visible and participates in
/// layout, painting, and hit testing.
///
/// This is a built-in `FatObj` field. Setting the `visible` field attaches a
/// `Visibility` wrapper which can hide the child from layout, painting, and
/// hit testing when disabled.
///
/// # Example
///
/// Hide a container by setting `visible` to `false`.
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(100., 100.),
///   background: Color::RED,
///   visible: false,
/// };
/// ```
#[derive(Default)]
pub struct Visibility {
  pub visible: bool,
}

impl Declare for Visibility {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for Visibility {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      @FocusScope {
        skip_descendants: pipe!(!$read(this).get_visible()),
        skip_host: pipe!(!$read(this).get_visible()),
        @VisibilityRender {
          display: pipe!($read(this).get_visible()),
          @ { child }
        }
      }
    }
    .into_widget()
  }
}

#[derive(Declare, Clone)]
struct VisibilityRender {
  display: bool,
}

impl_compose_child_for_wrap_render!(VisibilityRender);

impl WrapRender for VisibilityRender {
  #[inline]
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    if self.display || Self::allow_hidden_layout(ctx) {
      host.measure(clamp, ctx)
    } else {
      clamp.min
    }
  }

  #[inline]
  // `AllowHiddenPaint` can keep a hidden subtree participating in layout.
  // `size_affected_by_child` has no context parameter, so it cannot branch on
  // that provider dynamically. Forwarding to the host keeps relayout
  // propagation consistent with `measure`.
  fn size_affected_by_child(&self, host: &dyn Render) -> bool { host.size_affected_by_child() }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let allow_hidden_paint = Provider::of::<AllowHiddenPaint>(ctx).is_some_and(|v| v.0);
    if self.display || allow_hidden_paint {
      host.paint(ctx)
    } else {
      ctx.painter().apply_alpha(0.);
    }
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    if self.display {
      host.hit_test(ctx, pos)
    } else {
      HitTest { hit: false, can_hit_child: false }
    }
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }

  #[cfg(feature = "debug")]
  fn debug_type(&self) -> Option<&'static str> { Some("visibility") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({ "display": self.display }))
  }
}

impl VisibilityRender {
  #[inline]
  fn allow_hidden_layout(ctx: &MeasureCtx) -> bool {
    Provider::of::<AllowHiddenPaint>(ctx).is_some_and(|v| v.0)
  }
}

impl Visibility {
  #[inline]
  pub fn new(visible: bool) -> Self { Self { visible } }

  #[inline]
  fn get_visible(&self) -> bool { self.visible }
}

#[cfg(test)]
mod tests {
  use test_helper::split_value;

  use super::*;
  use crate::test_helper::*;

  #[test]
  fn visible_children_not_paint() {
    reset_test_env!();

    struct PainterHit(Stateful<i32>);

    impl Render for PainterHit {
      fn measure(&self, clamp: BoxClamp, _ctx: &mut MeasureCtx) -> Size { clamp.max }

      fn paint(&self, _ctx: &mut PaintingCtx) { *self.0.write() += 1; }
    }

    let hit = Stateful::new(0);
    let (visible, w_visible) = split_value(true);
    let hit2 = hit.clone_writer();
    let wnd = TestWindow::from_widget(container! {
      size: Size::splat(100.),
      visible: pipe!(*$read(visible)),
      @PainterHit(hit2.clone_writer())
    });

    wnd.draw_frame();
    assert_eq!(*hit.read(), 1);
    *w_visible.write() = false;
    wnd.draw_frame();
    assert_eq!(*hit.read(), 1);
  }

  #[test]
  fn allow_hidden_paint_keeps_hidden_layout() {
    reset_test_env!();

    let hidden_id = Stateful::new(None::<WidgetId>);
    let hidden_id_reader = hidden_id.clone_reader();
    let tail_id = Stateful::new(None::<WidgetId>);
    let tail_id_reader = tail_id.clone_reader();

    let wnd = TestWindow::from_widget(providers! {
      providers: [Provider::new(AllowHiddenPaint(true))],
      @MockMulti {
        @MockBox { size: Size::new(10., 10.) }
        @MockBox {
          visible: false,
          size: Size::new(20., 10.),
          on_mounted: move |e| *$write(hidden_id) = Some(e.current_target()),
        }
        @MockBox {
          size: Size::new(30., 10.),
          on_mounted: move |e| *$write(tail_id) = Some(e.current_target()),
        }
      }
    });

    wnd.draw_frame();

    let hidden_id = hidden_id_reader
      .read()
      .expect("hidden child should mount");
    let tail_id = tail_id_reader
      .read()
      .expect("tail child should mount");

    assert_eq!(wnd.widget_size(hidden_id), Some(Size::new(20., 10.)));
    assert_eq!(wnd.widget_pos(tail_id), Some(Point::new(30., 0.)));
  }
}
