use std::cell::RefCell;

use rxrust::{observable::boxed::LocalBoxedObservable, subscription::BoxedSubscription};

use crate::{prelude::*, ticker::FrameMsg};

/// Used by `GlobalAnchorX` and `GlobalAnchorY` to calculate the offset of the
/// top or the left position.
///
/// Returns Ok(offset) when calculate success.
/// Return Err(()) when failed, usually because the widget is dropped. Then will
/// cause the anchor to unsubscribe to refresh.
pub type AnchorOffsetFn = dyn Fn(&TrackId, &Rc<Window>) -> Result<f32, ()>;

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

/// A wrapper that anchors a child relative to global (window) coordinates.
///
/// Use the built-in fields `global_anchor_x` and `global_anchor_y` to attach
/// a `GlobalAnchor` that positions a widget based on global coordinates or
/// another widget's position.
///
/// Note: Anchoring a child outside its parent's bounds may affect hit-testing
/// and interactivity. When using global anchors, ensure the anchored widget
/// is placed in a container with sufficient space or rendered on a top-level
/// overlay layer to preserve expected layout and input behavior.
///
/// ### Example
///
/// When a button is clicked, show an overlay anchored to the button's top
/// center.
/// ``` rust no_run
/// use ribir::prelude::*;
/// let app = fn_widget! {
///   let mut button = @FatObj{ @FilledButton { @{ "click show overlay" } } };
///   let overlay = Overlay::new(
///     text! {
///       text: "anchor by global anchor",
///       global_anchor_x:
///         GlobalAnchorX::left_align_to($clone(button.track_id())),
///       global_anchor_y:
///         GlobalAnchorY::bottom_align_to($clone(button.track_id()))
///           .offset(*$read(button.layout_height()))
///     },
///     OverlayStyle {
///       auto_close_policy: AutoClosePolicy::TAP_OUTSIDE,
///       mask: None
///     });
///   @(button) {
///     margin: EdgeInsets::all(20.0),
///     on_tap: move |e| overlay.show(e.window()),
///   }
/// };
/// App::run(app);
/// ```
#[derive(Default)]
pub struct GlobalAnchor {
  /// the horizontal global anchor
  pub global_anchor_x: Option<GlobalAnchorX>,

  /// the vertical global anchor
  pub global_anchor_y: Option<GlobalAnchorY>,

  guard: RefCell<Option<SubscriptionGuard<BoxedSubscription>>>,
}

impl GlobalAnchorX {
  /// Init the horizontal offset from the HAnchor relative to the Window View.
  pub fn value(x: HAnchor) -> Self {
    Self::Once(Box::new(move |t, wnd: &Rc<Window>| {
      let wid = t.get().unwrap();
      if wid.is_dropped(wnd.tree()) {
        return Err(());
      }
      let size = wnd.widget_size(wid).unwrap();
      let wnd_size = wnd.size();
      Ok(x.into_pixel(size.width, wnd_size.width))
    }))
  }

  /// Init the global horizontal anchor from the custom function, which will
  /// return the horizontal offset relative to the global position
  pub fn custom(f: impl Fn(&TrackId, &Rc<Window>) -> Result<f32, ()> + 'static) -> Self {
    Self::Once(Box::new(f))
  }

  /// Init the global horizontal anchor, which will anchor the widget's
  /// horizontal position by placing its left edge align to the left edge of
  /// the specified widget (`target`)
  pub fn left_align_to(target: TrackId) -> Self {
    Self::Once(Box::new(move |host, wnd: &Rc<Window>| {
      let host_id = host.get().unwrap();
      let target_id = target.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target_id.is_dropped(wnd.tree()) {
        return Err(());
      }

      let target_parent = target_id.parent(wnd.tree()).unwrap();
      let target_pos = wnd.widget_pos(target_id).unwrap();
      let base = wnd.map_to_global(target_pos, target_parent).x;

      Ok(base)
    }))
  }

  /// Init the global horizontal anchor, which will anchor the widget's
  /// horizontal position by placing its center align to the center of the
  /// specified widget (`target`)
  pub fn center_align_to(target: TrackId) -> Self {
    Self::Once(Box::new(move |host, wnd: &Rc<Window>| {
      let host_id = host.get().unwrap();
      let target = target.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }
      let target_parent = target.parent(wnd.tree()).unwrap();
      let target_pos = wnd.widget_pos(target).unwrap();
      let base = wnd.map_to_global(target_pos, target_parent).x;

      let host_size = wnd.widget_size(host_id).unwrap_or_default();
      let target_size = wnd.widget_size(target).unwrap_or_default();

      Ok(base + (target_size.width - host_size.width) / 2.0)
    }))
  }

  /// Init the global horizontal anchor, which will anchor the widget's
  /// horizontal position by placing its right edge left to the right edge of
  /// the specified widget (`target`)
  pub fn right_align_to(track_id: TrackId) -> Self {
    Self::Once(Box::new(move |host, wnd: &Rc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }
      let target_parent = target.parent(wnd.tree()).unwrap();
      let target_pos = wnd.widget_pos(target).unwrap();
      let base = wnd.map_to_global(target_pos, target_parent).x;
      let host_size = wnd.widget_size(host_id).unwrap_or_default();
      let target_size = wnd.widget_size(target).unwrap_or_default();

      Ok(base + target_size.width - host_size.width)
    }))
  }

  /// Convert the once anchor into the always follow anchor
  pub fn always_follow(self) -> Self {
    match self {
      Self::Once(f) => Self::AlwaysFollow(f),
      _ => self,
    }
  }

  /// Add the offset to the anchor
  pub fn offset(self, offset: f32) -> Self {
    match self {
      Self::Once(f) => Self::Once(Box::new(move |host, wnd| f(host, wnd).map(|v| v + offset))),
      Self::AlwaysFollow(f) => {
        Self::AlwaysFollow(Box::new(move |host, wnd| f(host, wnd).map(|v| v + offset)))
      }
    }
  }

  fn is_once(&self) -> bool { matches!(self, Self::Once(_)) }

  fn offset_val(&self, host: &TrackId, wnd: &Rc<Window>) -> Result<f32, ()> {
    match self {
      Self::Once(f) => f(host, wnd),
      Self::AlwaysFollow(f) => f(host, wnd),
    }
  }
}

impl GlobalAnchorY {
  /// Init the global vertical anchor from VAnchor relative to the Window View.
  pub fn value(y: VAnchor) -> Self {
    Self::Once(Box::new(move |t, wnd: &Rc<Window>| {
      let wid = t.get().unwrap();
      if wid.is_dropped(wnd.tree()) {
        return Err(());
      }
      let size = wnd.widget_size(wid).unwrap();
      let wnd_size = wnd.size();
      Ok(y.into_pixel(size.height, wnd_size.height))
    }))
  }

  /// Init the global vertical anchor from the custom function, which will
  /// return the vertical offset relative to the global position
  pub fn custom(f: impl Fn(&TrackId, &Rc<Window>) -> Result<f32, ()> + 'static) -> Self {
    Self::Once(Box::new(f))
  }

  /// Init the global vertical anchor, which will anchor the widget's
  /// vertical position by placing its top edge align to the top edge of the
  /// specified widget (`target`).
  pub fn top_align_to(track_id: TrackId) -> Self {
    Self::Once(Box::new(move |host, wnd: &Rc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }

      let target_parent = target.parent(wnd.tree()).unwrap();
      let target_pos = wnd.widget_pos(target).unwrap();
      let base = wnd.map_to_global(target_pos, target_parent).y;

      Ok(base)
    }))
  }

  /// Init the global vertical anchor, which will anchor the widget's
  /// vertical position by placing its center vertical align to the center of
  /// the specified widget (`target`).
  pub fn center_align_to(track_id: TrackId) -> Self {
    Self::Once(Box::new(move |host, wnd: &Rc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }

      let target_parent = target.parent(wnd.tree()).unwrap();
      let target_pos = wnd.widget_pos(target).unwrap();
      let base = wnd.map_to_global(target_pos, target_parent).y;

      let host_size = wnd.widget_size(host_id).unwrap_or_default();
      let target_size = wnd.widget_size(target).unwrap_or_default();
      Ok(base + (target_size.height - host_size.height) / 2.0)
    }))
  }

  /// Init the global vertical anchor, which will anchor the widget's
  /// vertical position by placing its bottom edge align to the bottom edge of
  /// the specified widget (`target`).
  pub fn bottom_align_to(track_id: TrackId) -> Self {
    Self::Once(Box::new(move |host, wnd: &Rc<Window>| {
      let host_id = host.get().unwrap();
      let target = track_id.get().unwrap();
      if host_id.is_dropped(wnd.tree()) || target.is_dropped(wnd.tree()) {
        return Err(());
      }

      let target_parent = target.parent(wnd.tree()).unwrap();
      let target_pos = wnd.widget_pos(target).unwrap();
      let base = wnd.map_to_global(target_pos, target_parent).y;
      let host_size = wnd.widget_size(host_id).unwrap_or_default();
      let target_size = wnd.widget_size(target).unwrap_or_default();
      Ok(base + target_size.height - host_size.height)
    }))
  }

  /// convert the once anchor to the always follow anchor
  pub fn always_follow(self) -> Self {
    match self {
      Self::Once(f) => Self::AlwaysFollow(f),
      _ => self,
    }
  }

  /// Add the offset to the anchor
  pub fn offset(self, offset: f32) -> Self {
    match self {
      Self::Once(f) => Self::Once(Box::new(move |host, wnd| f(host, wnd).map(|v| v + offset))),
      Self::AlwaysFollow(f) => {
        Self::AlwaysFollow(Box::new(move |host, wnd| f(host, wnd).map(|v| v + offset)))
      }
    }
  }

  fn is_once(&self) -> bool { matches!(self, Self::Once(_)) }

  fn offset_val(&self, host: &TrackId, wnd: &Rc<Window>) -> Result<f32, ()> {
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
      let u = this.modifies()
        .subscribe(move |_| {
          let global = $writer(this);
          let relative = $writer(child.relative_anchor_widget());
          let track_id = $clone(child.track_id());
          apply_global_anchor(global, relative, track_id, wnd.clone());
        });

      @(child) {
        on_disposed: move |_| {
          u.unsubscribe();
          $read(this).guard.borrow_mut().take();
        }
      }
    }
    .into_widget()
    .dirty_on(modifies, DirtyPhase::Layout)
  }
}

fn apply_global_anchor(
  this: impl StateWriter<Value = GlobalAnchor>, anchor: impl StateWriter<Value = RelativeAnchor>,
  host: TrackId, wnd: Rc<Window>,
) {
  let tick_of_layout_ready = wnd
    .frame_tick_stream()
    .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));

  let anchor = anchor.clone_writer();
  let this_ref = this.read();
  let anchor_x = this_ref.global_anchor_x.as_ref();
  let anchor_y = this_ref.global_anchor_y.as_ref();

  let watch: LocalBoxedObservable<'static, _, _> =
    match (anchor_x.is_some_and(|x| x.is_once()), anchor_y.is_some_and(|y| y.is_once())) {
      (true, true) => tick_of_layout_ready.take(1).box_it(),
      _ => tick_of_layout_ready.box_it(),
    };
  let this = this.clone_writer();
  *this_ref.guard.borrow_mut() = Some(
    watch
      .subscribe(move |_| {
        let read_ref = this.read();
        let x = read_ref
          .global_anchor_x
          .as_ref()
          .map(|x| x.offset_val(&host, &wnd))
          .transpose();
        let y = read_ref
          .global_anchor_y
          .as_ref()
          .as_ref()
          .map(|y| y.offset_val(&host, &wnd))
          .transpose();

        if let (Ok(x), Ok(y)) = (x, y) {
          let pos = Point::new(x.unwrap_or_default(), y.unwrap_or_default());
          let id = host.get().unwrap();
          let parent = id.parent(wnd.tree()).unwrap();
          let pt = wnd.map_from_global(pos, parent);
          let mut anchor = anchor.write();

          let val = match (x.is_some(), y.is_some()) {
            (true, true) => Anchor::from_point(pt),
            (true, false) => Anchor::left(pt.x),
            (false, true) => Anchor::top(pt.y),
            (false, false) => Anchor::default(),
          };

          if anchor.anchor != val {
            anchor.anchor = val;
          }
        } else {
          let this = this.clone_reader();
          AppCtx::spawn_local(async move {
            this.read().guard.borrow_mut().take();
          });
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
        global_anchor_x: GlobalAnchorX::left_align_to($clone(parent.track_id())).offset(20.),
        global_anchor_y: GlobalAnchorY::top_align_to($clone(parent.track_id())).offset(10.),
      };

      let bottom_right = @MockBox {
        size: Size::new(10., 10.),
        global_anchor_x: GlobalAnchorX::right_align_to($clone(parent.track_id())).offset(-10.),
        global_anchor_y: GlobalAnchorY::bottom_align_to($clone(parent.track_id())).offset(-20.),
      };

      @(parent) {
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
