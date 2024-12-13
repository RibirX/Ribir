use std::cell::RefCell;

use ops::box_it::BoxOp;

use crate::{prelude::*, ticker::FrameMsg};

/// Used by `GlobalAnchorX` and `GlobalAnchorY` to calculate the offset of the
/// top or the left position.
///
/// Returns Ok(offset) when calculate success.
/// Return Err(()) when failed, usually because the widget is dropped. Then will
/// cause the anchor to unsubscribe to refresh.
pub type AnchorOffsetFn = dyn Fn(&TrackId, &Sc<Window>) -> Result<f32, ()>;

/// The horizontal global anchor
pub enum GlobalAnchorX {
  /// The Anchor will be recalculated once.
  /// Return the horizontal offset relative to the global position.
  Once(Box<AnchorOffsetFn>),

  /// The Anchor will be recalculated every frame
  /// Return the horizontal offset relative to the global position.
  AlwaysFollow(Box<AnchorOffsetFn>),
}

/// The Vertical global anchor
pub enum GlobalAnchorY {
  /// Return the vertical offset relative to the global position. The offset
  /// will be recalculated once
  Once(Box<AnchorOffsetFn>),

  /// Return the vertical offset relative to the global position. The offset
  /// will be recalculated every frame
  AlwaysFollow(Box<AnchorOffsetFn>),
}

/// This widget is used to anchor child constraints relative to the global
/// position. You can use it by builtin fields: `global_anchor_x` and
/// `global_anchor_y`.
///
/// It's important to note that if you anchor the child widget outside of its
/// parent, it may become unable to click, so ensure there is ample space within
/// the parent.
///
/// ### Example
/// ```no_run
/// use ribir::prelude::*;
/// let app = fn_widget! {
///   let mut button = @FilledButton {
///     @{ Label::new("click show overlay") }
///   };
///   let overlay = Overlay::new(
///     move || {
///       @Text {
///         text: "anchor by global anchor",
///         global_anchor_x:
///           GlobalAnchorX::center_align_to($button.track_id(), 0.),
///         global_anchor_y:
///           GlobalAnchorY::bottom_align_to(
///             $button.track_id(),
///             $button.layout_size().height
///           )
///        }.into_widget()
///     },
///     OverlayStyle {
///       auto_close_policy: AutoClosePolicy::TAP_OUTSIDE,
///       mask: None
///     });
///   @Container {
///     size: Size::new(200., 100.),
///     padding: EdgeInsets::all(20.0),
///     @ $button {
///       on_tap: move |e| overlay.show(e.window()),
///     }
///   }
/// }
/// App::run(app);
/// ```
pub struct GlobalAnchor {
  /// the horizontal global anchor
  pub global_anchor_x: GlobalAnchorX,

  /// the vertical global anchor
  pub global_anchor_y: GlobalAnchorY,

  guard: RefCell<Option<SubscriptionGuard<BoxSubscription<'static>>>>,
}

impl Default for GlobalAnchor {
  fn default() -> Self {
    Self {
      global_anchor_x: GlobalAnchorX::value(HAnchor::Left(0.0)),
      global_anchor_y: GlobalAnchorY::value(VAnchor::Top(0.0)),

      guard: Default::default(),
    }
  }
}

impl GlobalAnchorX {
  /// Init the horizontal offset from the HAnchor relative to the Window View.
  pub fn value(x: HAnchor) -> Self {
    match x {
      HAnchor::Left(offset) => Self::Once(Box::new(move |_, _| Ok(offset))),
      HAnchor::Right(offset) => Self::Once(Box::new(move |t, wnd: &Sc<Window>| {
        let wid = t.get().unwrap();
        if wid.is_dropped(wnd.tree()) {
          return Err(());
        }
        let size = wnd.widget_size(wid).unwrap();
        let wnd_size = wnd.size();
        Ok(wnd_size.width - size.width - offset)
      })),
    }
  }

  /// Init the global horizontal anchor from the custom function, which will
  /// return the horizontal offset relative to the global position
  pub fn custom(f: impl Fn(&TrackId, &Sc<Window>) -> Result<f32, ()> + 'static) -> Self {
    Self::Once(Box::new(f))
  }

  /// Init the global horizontal anchor, which will anchor the widget's
  /// horizontal position by placing its left edge right to the left edge of
  /// the specified widget (`target`) with the given relative pixel value
  pub fn left_align_to(target: TrackId, offset: f32) -> Self {
    Self::Once(Box::new(move |host, wnd: &Sc<Window>| {
      let host_id = host.get().unwrap();
      let target_id = target.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target_id.is_dropped(wnd.tree()) {
        return Err(());
      }
      let base = wnd.map_to_global(Point::zero(), target_id).x;
      Ok(offset + base)
    }))
  }

  /// Init the global horizontal anchor, which will anchor the widget's
  /// horizontal position by placing its center right to the center of the
  /// specified widget (`target`) with the given relative pixel value
  pub fn center_align_to(track_id: TrackId, offset: f32) -> Self {
    Self::Once(Box::new(move |host, wnd: &Sc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }
      let base = wnd.map_to_global(Point::zero(), target).x;
      let host_size = wnd.widget_size(host_id).unwrap_or_default();
      let target_size = wnd.widget_size(target).unwrap_or_default();
      Ok(base + (target_size.width - host_size.width) / 2.0 + offset)
    }))
  }

  /// Init the global horizontal anchor, which will anchor the widget's
  /// horizontal position by placing its right edge left to the right edge of
  /// the specified widget (`target`) with the given relative pixel value
  pub fn right_align_to(track_id: TrackId, offset: f32) -> Self {
    Self::Once(Box::new(move |host, wnd: &Sc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }
      let base = wnd.map_to_global(Point::zero(), target).x;
      let host_size = wnd.widget_size(host_id).unwrap_or_default();
      let target_size = wnd.widget_size(target).unwrap_or_default();
      Ok(base + target_size.width - host_size.width - offset)
    }))
  }

  /// Convert the once anchor into the always follow anchor
  pub fn always_follow(self) -> Self {
    match self {
      Self::Once(f) => Self::AlwaysFollow(f),
      _ => self,
    }
  }

  fn is_once(&self) -> bool { matches!(self, Self::Once(_)) }

  fn offset(&self, host: &TrackId, wnd: &Sc<Window>) -> Result<f32, ()> {
    match self {
      Self::Once(f) => f(host, wnd),
      Self::AlwaysFollow(f) => f(host, wnd),
    }
  }
}

impl GlobalAnchorY {
  /// Init the global vertical anchor from VAnchor relative to the Window View.
  pub fn value(y: VAnchor) -> Self {
    match y {
      VAnchor::Top(offset) => Self::Once(Box::new(move |_, _| Ok(offset))),
      VAnchor::Bottom(offset) => Self::Once(Box::new(move |t, wnd: &Sc<Window>| {
        let wid = t.get().unwrap();
        if wid.is_dropped(wnd.tree()) {
          return Err(());
        }
        let size = wnd.widget_size(wid).unwrap();
        let wnd_size = wnd.size();
        Ok(wnd_size.height - size.height - offset)
      })),
    }
  }

  /// Init the global vertical anchor from the custom function, which will
  /// return the vertical offset relative to the global position
  pub fn custom(f: impl Fn(&TrackId, &Sc<Window>) -> Result<f32, ()> + 'static) -> Self {
    Self::Once(Box::new(f))
  }

  /// Init the global vertical anchor, which will anchor the widget's
  /// vertical position by placing its top edge down to the top edge of the
  /// specified widget (`target`) with the given relative pixel value
  pub fn top_align_to(track_id: TrackId, offset: f32) -> Self {
    Self::Once(Box::new(move |host, wnd: &Sc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }
      let y = wnd.map_to_global(Point::zero(), target).y;
      Ok(offset + y)
    }))
  }

  /// Init the global vertical anchor, which will anchor the widget's
  /// vertical position by placing its center down to the center of the
  /// specified widget (`target`) with the given relative pixel value
  pub fn center_align_to(track_id: TrackId, offset: f32) -> Self {
    Self::Once(Box::new(move |host, wnd: &Sc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }
      let y = wnd.map_to_global(Point::zero(), target).y;
      let host_size = wnd.widget_size(host_id).unwrap_or_default();
      let target_size = wnd.widget_size(target).unwrap_or_default();
      Ok(y + (target_size.height - host_size.height) / 2.0 + offset)
    }))
  }

  /// Init the global vertical anchor, which will anchor the widget's
  /// vertical position by placing its bottom edge up to the bottom edge of
  /// the specified widget (`target`) with the given relative pixel value
  pub fn bottom_align_to(track_id: TrackId, offset: f32) -> Self {
    Self::Once(Box::new(move |host, wnd: &Sc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }
      let y = wnd.map_to_global(Point::zero(), target).y;
      let host_size = wnd.widget_size(host_id).unwrap_or_default();
      let target_size = wnd.widget_size(target).unwrap_or_default();
      Ok(y + target_size.height - host_size.height - offset)
    }))
  }

  /// convert the once anchor to the always follow anchor
  pub fn always_follow(self) -> Self {
    match self {
      Self::Once(f) => Self::AlwaysFollow(f),
      _ => self,
    }
  }

  fn is_once(&self) -> bool { matches!(self, Self::Once(_)) }

  fn offset(&self, host: &TrackId, wnd: &Sc<Window>) -> Result<f32, ()> {
    match self {
      Self::Once(f) => f(host, wnd),
      Self::AlwaysFollow(f) => f(host, wnd),
    }
  }
}

impl Declare for GlobalAnchor {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for GlobalAnchor {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let modifies = this.raw_modifies();
    fn_widget! {
      let wnd = BuildCtx::get().window();
      let mut child = FatObj::new(child);
      let this2 = this.clone_writer();
      let anchor_widget = child.get_relative_anchor_widget().clone_writer();
      let u = this.modifies()
          .subscribe(move |_| {
            apply_global_anchor(&this2, &anchor_widget, $child.track_id(), wnd.clone());
          });

      @ $child {
        on_disposed: move |_| {
          u.unsubscribe();
          $this.guard.borrow_mut().take();
        }
      }
    }
    .into_widget()
    .on_build(move |id| id.dirty_on(modifies))
  }
}

fn apply_global_anchor(
  this: &impl StateWriter<Value = GlobalAnchor>, anchor: &impl StateWriter<Value = RelativeAnchor>,
  host: TrackId, wnd: Sc<Window>,
) {
  let tick_of_layout_ready = wnd
    .frame_tick_stream()
    .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));

  let anchor = anchor.clone_writer();
  let this_ref = this.read();
  let watch: BoxOp<'static, _, _> =
    match (this_ref.global_anchor_x.is_once(), this_ref.global_anchor_y.is_once()) {
      (true, true) => tick_of_layout_ready.take(1).box_it(),
      _ => tick_of_layout_ready.box_it(),
    };
  let this = this.clone_writer();
  *this_ref.guard.borrow_mut() = Some(
    watch
      .subscribe(move |_| {
        let read_ref = this.read();
        let x = read_ref.global_anchor_x.offset(&host, &wnd);
        let y = read_ref.global_anchor_y.offset(&host, &wnd);

        if let (Ok(x), Ok(y)) = (x, y) {
          let id = host.get().unwrap();
          let parent = id.parent(wnd.tree()).unwrap();
          let pt = wnd.map_from_global(Point::new(x, y), parent);
          let mut anchor = anchor.write();
          let val = Anchor { x: Some(HAnchor::Left(pt.x)), y: Some(VAnchor::Top(pt.y)) };
          if anchor.anchor != val {
            anchor.anchor = val;
          }
        } else {
          read_ref.guard.borrow_mut().take();
        }
      })
      .unsubscribe_when_dropped(),
  );
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  const WND_SIZE: Size = Size::new(100., 100.);

  widget_layout_test!(
    global_anchor,
    WidgetTester::new(fn_widget! {
      let mut parent = @MockBox {
        anchor: Anchor::left_top(10., 10.),
        size: Size::new(50., 50.),
      };

      let top_left = @MockBox {
        size: Size::new(10., 10.),
        global_anchor_x: GlobalAnchorX::left_align_to($parent.track_id(), 20.),
        global_anchor_y: GlobalAnchorY::top_align_to($parent.track_id(), 10.),
      };

      let bottom_right = @MockBox {
        size: Size::new(10., 10.),
        global_anchor_x: GlobalAnchorX::right_align_to($parent.track_id(), 10.),
        global_anchor_y: GlobalAnchorY::bottom_align_to($parent.track_id(), 20.),
      };

      @ $parent {
        @MockStack {
          @ { top_left }
          @ { bottom_right }
        }
      }
    })
    .with_wnd_size(WND_SIZE),
    LayoutCase::new(&[0, 0, 0]).with_pos(Point::new(20., 10.)),
    LayoutCase::new(&[0, 0, 1]).with_pos(Point::new(30., 20.))
  );
}
