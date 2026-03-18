use std::cell::OnceCell;

use ribir_core::{prelude::*, wrap_render::*};
use smallvec::SmallVec;

use super::{AnimateMatch, Interruption, MatchCases, OptionalTransitionSelector};

/// A visibility wrapper that animates a static widget's enter/leave
/// transitions.
///
/// `AnimatedVisibility` is the animated counterpart to the builtin `visible`
/// property. Use [`show`](AnimatedVisibility::show) as the single visibility
/// input when you need enter/leave animations. Unlike the builtin `visible`,
/// this wrapper can keep painting hidden content while a leave animation is
/// still running.
pub struct AnimatedVisibility<S: AnimateState + 'static> {
  show: bool,
  animate_match: OnceCell<AnimateMatch<bool, S>>,
}

pub struct AnimatedVisibilityDeclarer<S: AnimateState + 'static> {
  fat_obj: FatObj<()>,
  show: Option<PipeValue<bool>>,
  cases: Option<MatchCases<bool, S>>,
  enter: Option<Box<dyn Transition>>,
  leave: Option<Box<dyn Transition>>,
  interruption: Option<Interruption>,
}

impl<S: AnimateState + 'static> Declare for AnimatedVisibility<S> {
  type Builder = AnimatedVisibilityDeclarer<S>;

  fn declarer() -> Self::Builder {
    AnimatedVisibilityDeclarer {
      fat_obj: FatObj::new(()),
      show: None,
      cases: None,
      enter: None,
      leave: None,
      interruption: None,
    }
  }
}

/// Macro to create an [`AnimatedVisibility`] as the root of a function widget.
#[macro_export]
macro_rules! animated_visibility {
  ($($t: tt)*) => { fn_widget! { @AnimatedVisibility { $($t)* } } };
}
pub use animated_visibility;

impl<S> AnimatedVisibilityDeclarer<S>
where
  S: AnimateState<Value: Clone> + 'static,
{
  #[track_caller]
  pub fn with_show(&mut self, show: Pipe<bool>) -> &mut Self {
    assert!(self.show.is_none(), "AnimatedVisibility: `show` is already set");
    self.show = Some(show.r_into());
    self
  }

  pub fn with_cases(&mut self, cases: MatchCases<bool, S>) -> &mut Self {
    assert!(self.cases.is_none(), "AnimatedVisibility: `cases` is already set");
    self.cases = Some(cases);
    self
  }

  pub fn with_enter(&mut self, transition: impl Transition + 'static) -> &mut Self {
    assert!(self.enter.is_none(), "AnimatedVisibility: `enter` is already set");
    self.enter = Some(transition.into_box());
    self
  }

  pub fn with_leave(&mut self, transition: impl Transition + 'static) -> &mut Self {
    assert!(self.leave.is_none(), "AnimatedVisibility: `leave` is already set");
    self.leave = Some(transition.into_box());
    self
  }

  pub fn with_interruption(&mut self, interruption: Interruption) -> &mut Self {
    assert!(self.interruption.is_none(), "AnimatedVisibility: `interruption` is already set");
    self.interruption = Some(interruption);
    self
  }
}

impl<S> ObjDeclarer for AnimatedVisibilityDeclarer<S>
where
  S: AnimateState<Value: Clone> + 'static,
{
  type Target = FatObj<Stateful<AnimatedVisibility<S>>>;

  fn finish(self) -> Self::Target {
    let (show, show_stream) = self
      .show
      .expect("AnimatedVisibility requires `show`")
      .unzip();
    let enter = self.enter;
    let leave = self.leave;

    let host = Stateful::new(AnimatedVisibility { show, animate_match: OnceCell::new() });
    let animate_match = AnimateMatch::observe(
      host.part_watcher(|this| PartRef::from_value(this.show)),
      self
        .cases
        .expect("AnimatedVisibility requires `cases`"),
      OptionalTransitionSelector::new(move |from: &bool, to: &bool| match (*from, *to) {
        (false, true) => enter.clone(),
        (true, false) => leave.clone(),
        _ => None,
      }),
      self.interruption.unwrap_or_default(),
    );
    let host_ref = host.write();
    if host_ref.animate_match.set(animate_match).is_err() {
      unreachable!("AnimatedVisibility animate_match should only be initialized once");
    }
    drop(host_ref);

    let mut fat = self.fat_obj.map(|_| host);
    if let Some(show_stream) = show_stream {
      let mut subscriptions = SmallVec::<[BoxedSubscription; 1]>::new();
      let host = fat.host().clone_writer();
      let subscription = show_stream.subscribe(move |show| host.write().show = show);
      subscriptions.push(subscription);

      fat.on_disposed(move |_| {
        for sub in subscriptions {
          sub.unsubscribe();
        }
      });
    }

    fat
  }
}

impl<S: AnimateState + 'static> std::ops::Deref for AnimatedVisibilityDeclarer<S> {
  type Target = FatObj<()>;

  fn deref(&self) -> &Self::Target { &self.fat_obj }
}

impl<S: AnimateState + 'static> std::ops::DerefMut for AnimatedVisibilityDeclarer<S> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.fat_obj }
}

impl<S: AnimateState + 'static> AnimatedVisibility<S> {
  fn animate_match(&self) -> &AnimateMatch<bool, S> {
    self
      .animate_match
      .get()
      .expect("AnimatedVisibility animate_match should be initialized in finish")
  }

  fn is_leaving(&self) -> bool { !self.show && self.animate_match().is_running() }

  /// Internal method to trigger a widget rebuild when the animation state
  /// changes.
  fn touch_to_rebuild(&mut self) {}
}

impl<'c, S> ComposeChild<'c> for AnimatedVisibility<S>
where
  S: AnimateState<Value: Clone> + 'static,
{
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let animate_match = this.read().animate_match().clone();
    let leave_running = animate_match.running_watcher();

    fn_widget! {
      // When the leave animation starts or finishes, we need to rebuild to
      // toggle the "hidden paint" or layout visibility.
      let leave_subscription = leave_running
        .modifies()
        .subscribe(move |_| $write(this).shallow().touch_to_rebuild());

      @FocusScope {
        skip_descendants: pipe!(!$read(this).show),
        skip_host: pipe!(!$read(this).show),
        on_disposed: move |_| leave_subscription.unsubscribe(),
        @WrapRender::combine_child(this, child)
      }
    }
    .into_widget()
  }
}

impl<S> WrapRender for AnimatedVisibility<S>
where
  S: AnimateState<Value: Clone> + 'static,
{
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    if self.show {
      return host.measure(clamp, ctx);
    }

    if self.is_leaving() {
      return Self::with_hidden_paint(ctx, |ctx| host.measure(clamp, ctx));
    }

    clamp.min
  }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool {
    if self.show || self.is_leaving() { host.size_affected_by_child() } else { false }
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    if self.show {
      return host.paint(ctx);
    }

    if self.is_leaving() {
      return Self::with_hidden_paint(ctx, |ctx| host.paint(ctx));
    }

    ctx.painter().apply_alpha(0.);
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    if self.show { host.hit_test(ctx, pos) } else { HitTest { hit: false, can_hit_child: false } }
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }

  #[cfg(feature = "debug")]
  fn debug_type(&self) -> Option<&'static str> { Some("animated_visibility") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({
      "show": self.show,
      "leaving": self.is_leaving(),
    }))
  }
}

impl<S: AnimateState + 'static> AnimatedVisibility<S> {
  fn with_hidden_paint<Ctx, T>(ctx: &mut Ctx, f: impl FnOnce(&mut Ctx) -> T) -> T
  where
    Ctx: AsMut<ProviderCtx>,
  {
    let mut provider = Provider::new(AllowHiddenPaint(true));
    provider.setup(ctx.as_mut());
    let value = f(ctx);
    provider.restore(ctx.as_mut());
    value
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc, time::Instant};

  use ribir_core::{reset_test_env, test_helper::*, window::WindowFlags};

  use super::*;
  use crate::prelude::*;

  const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);

  fn eventually(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;

    loop {
      if predicate() {
        return true;
      }
      if Instant::now() >= deadline {
        return false;
      }
      std::thread::sleep(TEST_POLL_INTERVAL);
    }
  }

  struct PainterHit(Stateful<i32>);

  impl Render for PainterHit {
    fn measure(&self, clamp: BoxClamp, _ctx: &mut MeasureCtx) -> Size { clamp.max }

    fn paint(&self, _ctx: &mut PaintingCtx) { *self.0.write() += 1; }
  }

  struct OpacityRecorder {
    opacity: Stateful<f32>,
    frames: Stateful<Vec<f32>>,
  }

  impl Render for OpacityRecorder {
    fn measure(&self, clamp: BoxClamp, _ctx: &mut MeasureCtx) -> Size { clamp.max }

    fn paint(&self, _ctx: &mut PaintingCtx) { self.frames.write().push(*self.opacity.read()); }
  }

  #[test]
  fn hidden_paint_runs_during_leave() {
    reset_test_env!();

    let show = Stateful::new(true);
    let hit = Stateful::new(0);
    let hit2 = hit.clone_writer();

    let w = fn_widget! {
      let mut painter = FatObj::new(PainterHit(hit2.clone_writer()));
      let opacity = painter.opacity();
      @AnimatedVisibility {
        show: pipe!(*$read(show)),
        cases: cases! {
          state: opacity,
          true => 1.0,
          false => 0.0,
        },
        leave: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(40),
        },
        @ { painter }
      }
    };

    let wnd = TestWindow::new(w, Size::new(100., 100.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    assert_eq!(*hit.read(), 1);

    *show.write() = false;
    wnd.draw_frame();
    assert!(*hit.read() >= 2, "hidden child should still paint during leave");

    assert!(
      eventually(Duration::from_millis(250), || {
        let painted_before = *hit.read();
        wnd.draw_frame();
        *hit.read() == painted_before
      }),
      "leave end should stop hidden painting"
    );
  }

  #[test]
  fn leave_keeps_layout_until_animation_finishes() {
    reset_test_env!();

    let show = Stateful::new(true);
    let mounted_id = Stateful::new(None::<WidgetId>);
    let mounted_id_reader = mounted_id.clone_reader();

    let w = fn_widget! {
      @Column {
        x: AnchorX::center(),
        y: AnchorY::center(),
        @MockBox { size: Size::new(80., 20.) }
        @ {
          let mut item = @MockBox {
            size: Size::new(120., 60.),
            on_mounted: move |e| *$write(mounted_id) = Some(e.current_target()),
          };
          let opacity = item.opacity();

          @AnimatedVisibility {
            show: pipe!(*$read(show)),
            cases: cases! {
              state: opacity,
              true => 1.0,
              false => 0.0,
            },
            leave: EasingTransition {
              easing: easing::LINEAR,
              duration: Duration::from_millis(40),
            },
            @ { item }
          }
        }
        @MockBox { size: Size::new(90., 20.) }
      }
    };

    let wnd = TestWindow::new(w, Size::new(400., 300.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    let id = mounted_id_reader
      .read()
      .expect("animated visibility child should mount before hide");
    let original_pos = wnd
      .widget_pos(id)
      .expect("child should have layout before hide");
    let original_size = wnd
      .widget_size(id)
      .expect("child should have size before hide");

    *show.write() = false;
    wnd.draw_frame();

    assert_eq!(
      wnd.widget_pos(id),
      Some(original_pos),
      "leave animation should keep original position"
    );
    assert_eq!(
      wnd.widget_size(id),
      Some(original_size),
      "leave animation should keep original size"
    );

    assert!(
      eventually(Duration::from_millis(250), || {
        wnd.draw_frame();
        wnd.widget_size(id) == Some(Size::zero())
      }),
      "layout should collapse after leave animation completes"
    );
  }

  #[test]
  fn enter_runs_on_show_again() {
    reset_test_env!();

    let show = Stateful::new(false);
    let opacity = Stateful::new(1.0_f32);
    let lerp_hits = Rc::new(Cell::new(0));
    let lerp_hits_reader = lerp_hits.clone();

    let w = fn_widget! {
      let lerp_hits = lerp_hits.clone();
      @AnimatedVisibility {
        show: pipe!(*$read(show)),
        cases: cases! {
          state: CustomLerpState::from_writer(opacity.clone_writer(), move |from, to, rate| {
            lerp_hits.set(lerp_hits.get() + 1);
            from.lerp(to, rate)
          }),
          true => 1.0,
          false => 0.0,
        },
        enter: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(200),
        },
        @MockBox {
          size: Size::new(100., 100.),
          opacity: pipe!(*$read(opacity)),
        }
      }
    };

    let wnd = TestWindow::new(w, Size::new(100., 100.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();
    *show.write() = true;
    wnd.draw_frame();
    assert!(
      eventually(Duration::from_millis(250), || {
        wnd.draw_frame();
        lerp_hits_reader.get() > 0
      }),
      "enter animation should advance through the custom lerp state"
    );
  }

  #[test]
  fn show_again_restores_leave_state_without_enter() {
    reset_test_env!();

    let show = Stateful::new(true);
    let opacity = Stateful::new(1.0_f32);
    let opacity_reader = opacity.clone_reader();

    let w = fn_widget! {
      @AnimatedVisibility {
        show: pipe!(*$read(show)),
        cases: cases! {
          state: opacity.clone_writer(),
          true => 1.0,
          false => 0.0,
        },
        leave: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(40),
        },
        @MockBox {
          size: Size::new(100., 100.),
          opacity: pipe!(*$read(opacity)),
        }
      }
    };

    let wnd = TestWindow::new(w, Size::new(100., 100.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();

    *show.write() = false;
    wnd.draw_frame();
    assert!(
      eventually(Duration::from_millis(250), || {
        wnd.draw_frame();
        *opacity_reader.read() == 0.0
      }),
      "leave animation should settle to the hidden target before showing again"
    );

    *show.write() = true;
    wnd.draw_frame();
    assert!(
      *opacity_reader.read() > 0.0,
      "showing again should reverse from the current leave interpolation"
    );
  }

  #[test]
  fn reversing_leave_continues_without_opacity_jump() {
    reset_test_env!();

    let show = Stateful::new(true);
    let opacity = Stateful::new(1.0_f32);
    let frames = Stateful::new(Vec::new());
    let frames_reader = frames.clone_reader();

    let w = fn_widget! {
      let recorder = FatObj::new(OpacityRecorder {
        opacity: opacity.clone_writer(),
        frames: frames.clone_writer(),
      });
      @AnimatedVisibility {
        show: pipe!(*$read(show)),
        cases: cases! {
          state: opacity.clone_writer(),
          true => 1.0,
          false => 0.0,
        },
        enter: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(200),
        },
        leave: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(200),
        },
        interruption: Interruption::Fluid,
        @ { recorder }
      }
    };

    let wnd = TestWindow::new(w, Size::new(100., 100.), WindowFlags::ANIMATIONS);
    wnd.draw_frame();

    *show.write() = false;
    wnd.draw_frame();
    assert!(
      eventually(Duration::from_millis(300), || {
        wnd.draw_frame();
        frames_reader
          .read()
          .last()
          .is_some_and(|opacity| *opacity < 0.95)
      }),
      "leave animation should progress before reversing"
    );
    let opacity_during_leave = *frames_reader
      .read()
      .last()
      .expect("leave frame should be painted before reversing");
    assert!(
      opacity_during_leave < 0.95,
      "leave animation should progress before reversing, got {opacity_during_leave}"
    );

    *show.write() = true;
    wnd.draw_frame();
    let opacity_when_reversed = *frames_reader
      .read()
      .last()
      .expect("reversed enter frame should be painted");

    assert!(
      (opacity_when_reversed - opacity_during_leave).abs() < 0.2,
      "reversing leave should continue from the current interpolated opacity, \
       before={opacity_during_leave}, after={opacity_when_reversed}"
    );
  }
}
