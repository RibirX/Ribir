use ribir::{
  core::{reset_test_env, test_helper::*},
  prelude::*,
};

#[test]
fn declarer_smoke() {
  reset_test_env!();
  // empty struct
  #[declare]
  struct A;

  let _: FatObj<A> = A::declarer().finish();

  #[declare]
  struct B {
    a: f32,
    b: i32,
  }

  let mut b = <B as Declare>::declarer();
  b.with_a(1.).with_b(1);
  let b = b.finish();
  assert_eq!(b.read().a, 1.);
  assert_eq!(b.read().b, 1);
}

#[test]
fn declare_attr_with_args() {
  reset_test_env!();
  #[declare(validate)]
  struct T {
    a: f32,
  }

  impl T {
    fn declare_validate(self) -> Result<Self, String> {
      if self.a > 0. { Ok(self) } else { Err("a must > 0".to_string()) }
    }
  }

  let mut t = T::declarer();
  t.with_a(1.);
  let _ = t.finish();
}

#[test]
#[should_panic = "Validation failed"]
fn declare_attr_validate_panic() {
  reset_test_env!();
  #[declare(validate)]
  struct T {
    a: f32,
  }

  impl T {
    fn declare_validate(self) -> Result<Self, String> {
      if self.a > 0. { Ok(self) } else { Err("a must > 0".to_string()) }
    }
  }

  let mut t = T::declarer();
  t.with_a(-1.);
  let _ = t.finish();
}

#[test]
#[should_panic = "Required field `T::a` not set"]
fn panic_if_miss_require_field() {
  reset_test_env!();
  #[declare]
  struct T {
    a: f32,
  }

  let _ = <T as Declare>::declarer().finish();
}

#[test]

fn default_field() {
  reset_test_env!();
  #[declare]
  struct DefaultDeclare {
    #[declare(default)]
    a: f32,
  }

  let t = <DefaultDeclare as Declare>::declarer().finish();
  assert_eq!(t.read().a, 0.);
}

#[test]

fn default_field_with_value() {
  reset_test_env!();
  #[declare]
  struct DefaultWithValue {
    #[declare(default = "hi!")]
    text: &'static str,
  }

  let t = <DefaultWithValue as Declare>::declarer().finish();
  assert_eq!(t.read().text, "hi!");
}

#[test]
fn declarer_simple_attr() {
  reset_test_env!();
  #[declare(simple)]
  struct Simple {
    a: f32,
    b: i32,
  }

  let mut s = Simple::declarer();
  s.with_a(1.).with_b(1);
  let s = s.finish();
  assert_eq!(s.read().a, 1.);
  assert_eq!(s.read().b, 1);
}

#[test]
fn unified_declare_simple() {
  reset_test_env!();
  #[declare(simple)]
  struct Simple {
    a: f32,
    b: i32,
  }

  let mut s = Simple::declarer();
  s.with_a(1.).with_b(1);
  let s = s.finish();
  assert_eq!(s.read().a, 1.);
  assert_eq!(s.read().b, 1);
}

#[test]
fn unified_declare_stateless() {
  reset_test_env!();
  #[declare(simple, stateless)]
  struct Stateless {
    a: f32,
  }

  let mut s = Stateless::declarer();
  s.with_a(1.);
  let s: Stateless = s.finish();
  assert_eq!(s.a, 1.);
}

#[test]
fn unified_declare_full_stateless() {
  reset_test_env!();
  #[declare(stateless)]
  struct FullStateless {
    a: f32,
  }

  let mut s = FullStateless::declarer();
  s.with_a(1.);
  let mut s = s.finish();
  // s should be FatObj<FullStateless>
  assert_eq!(s.a, 1.);
  let _ = s.with_margin(ribir::core::builtin_widgets::EdgeInsets::all(10.));
}

#[test]
fn strict_event_interaction() {
  reset_test_env!();
  #[declare]
  struct Interaction {
    #[declare(strict, event = f32)]
    a: f32,
  }

  let mut it = Interaction::declarer();
  it.with_a(TwoWayValue::Pipe(PipeValue::Value(1.)));
  let it = it.finish();
  assert_eq!(it.read().a, 1.);
}

#[test]
fn normal_strict_field() {
  reset_test_env!();
  #[declare]
  struct S {
    #[declare(strict)]
    a: f32,
  }

  let mut s = S::declarer();
  s.with_a(1.0);
  let s = s.finish();
  assert_eq!(s.read().a, 1.0);
}

use ribir::prelude::WidgetCtx;

#[test]
fn two_way_complete_set() {
  reset_test_env!();

  #[derive(Clone)]
  struct SimpleEvent(f32);
  impl From<SimpleEvent> for f32 {
    fn from(v: SimpleEvent) -> Self { v.0 }
  }

  #[derive(Clone)]
  struct FieldEvent {
    v: f32,
  }

  #[derive(Clone)]
  struct MethodEvent(f32);
  impl MethodEvent {
    fn get_v(&self) -> f32 { self.0 }
  }

  #[derive(Clone)]
  struct OptEvent {
    v: Option<f32>,
  }

  #[derive(Clone)]
  struct DeepEvent {
    a: Inner,
  }
  #[derive(Clone)]
  struct Inner {
    b: f32,
  }

  #[declare]
  struct TwoWayTest {
    #[declare(event = SimpleEvent)]
    a: f32,
    #[declare(event = FieldEvent.v)]
    b: f32,
    #[declare(event = MethodEvent.get_v())]
    c: f32,
    #[declare(event = OptEvent.v)]
    d: f32,
    #[declare(event = DeepEvent.a.b)]
    e: f32,
  }

  impl Compose for TwoWayTest {
    fn compose(_this: impl StateWriter<Value = Self>) -> Widget<'static> {
      fn_widget! {
        @MockBox {
          size: Size::new(100., 100.),
          on_mounted: move |e| {
            let id = e.current_target();
            let wnd = e.window();
            wnd.bubble_custom_event(id, SimpleEvent(1.));
            wnd.bubble_custom_event(id, FieldEvent { v: 2. });
            wnd.bubble_custom_event(id, MethodEvent(3.));
            wnd.bubble_custom_event(id, OptEvent { v: Some(4.) });
            wnd.bubble_custom_event(id, DeepEvent { a: Inner { b: 5. } });
          }
        }
      }
      .into_widget()
    }
  }

  let a = Stateful::new(0.0f32);
  let b = Stateful::new(0.0f32);
  let c = Stateful::new(0.0f32);
  let d = Stateful::new(0.0f32);
  let e = Stateful::new(0.0f32);

  let wnd = TestWindow::new_with_size(
    fn_widget! {
      let mut w = TwoWayTest::declarer();
      w.with_a(TwoWay::new($writer(a)))
        .with_b(TwoWay::new($writer(b)))
        .with_c(TwoWay::new($writer(c)))
        .with_d(TwoWay::new($writer(d)))
        .with_e(TwoWay::new($writer(e)));
      w.finish()
    },
    Size::new(100., 100.),
  );
  wnd.draw_frame();

  assert_eq!(*a.read(), 1.);
  assert_eq!(*b.read(), 2.);
  assert_eq!(*c.read(), 3.);
  assert_eq!(*d.read(), 4.);
  assert_eq!(*e.read(), 5.);
}

#[test]
fn eager_mode_basic() {
  reset_test_env!();

  #[derive(Default)]
  #[declare(eager)]
  struct EagerWidget {
    #[declare(default = 42.0)]
    value: f32,
  }

  let w = EagerWidget::declarer().finish();
  assert_eq!(w.read().value, 42.0);
}

#[test]
fn eager_mode_with_setter() {
  reset_test_env!();

  #[derive(Default)]
  #[declare(eager)]
  struct EagerWidget {
    #[declare(default)]
    value: f32,
  }

  let mut d = EagerWidget::declarer();
  d.with_value(100.0);
  let w = d.finish();
  assert_eq!(w.read().value, 100.0);
}

#[test]
fn eager_mode_fallback_to_default() {
  reset_test_env!();

  #[derive(Default)]
  #[declare(eager)]
  struct EagerWidget {
    // No explicit default - uses Default::default()
    #[declare(default)]
    value: f32,
  }

  let w = EagerWidget::declarer().finish();
  assert_eq!(w.read().value, 0.0); // f32::default() is 0.0
}

#[test]
fn eager_mode_with_builtin_widget() {
  reset_test_env!();

  #[derive(Default)]
  #[declare(eager)]
  struct EagerWidget {
    #[declare(default = 10.0)]
    value: f32,
  }

  let mut d = EagerWidget::declarer();
  d.with_value(50.0);
  d.with_margin(ribir::core::builtin_widgets::EdgeInsets::all(10.));
  let w = d.finish();
  assert_eq!(w.read().value, 50.0);
}

#[test]
fn eager_mode_with_pipe() {
  reset_test_env!();

  #[derive(Default)]
  #[declare(eager)]
  struct EagerWidget {
    #[declare(default = 10.0)]
    value: f32,
  }

  // Create a pipe source
  let src = Stateful::new(42.0f32);
  let pipe = Pipe::from_watcher(src.clone_watcher());

  let mut d = EagerWidget::declarer();
  // with_value accepts PipeValue, which Pipe implements RInto for
  d.with_value(PipeValue::Pipe { init_value: 42.0, pipe });
  let w = d.finish();

  // Verify initial value
  assert_eq!(w.read().value, 42.0);
}

/// Example: Eager mode with pipe support for partial field initialization.
///
/// This demonstrates how to implement partial setters that support both plain
/// values and reactive pipes. The `width` and `height` fields can accept:
/// - Plain values: `@SizedBox { width: 100.0, height: 50.0 }`
/// - Pipe bindings: `@SizedBox { width: pipe!($w), height: pipe!($h) }`
#[test]
fn eager_mode_partial_field_with_pipe() {
  reset_test_env!();

  use ribir::core::prelude::{Pipe, PipeValue, RInto, Size};

  #[derive(Default)]
  #[declare(eager)]
  struct SizedBox {
    #[declare(default)]
    size: Size,
  }

  // Implement custom partial setters with pipe support
  impl SizedBoxDeclarer {
    /// Sets the width component, supporting both values and pipes
    fn with_width<K: ?Sized>(&mut self, width: impl RInto<PipeValue<f32>, K>) -> &mut Self {
      let host = self.host().clone_writer();
      let mix = self.mix_builtin_widget();
      mix.init_sub_widget(width, &host, |w: &mut SizedBox, v| w.size.width = v);
      self
    }

    /// Sets the height component, supporting both values and pipes
    fn with_height<K: ?Sized>(&mut self, height: impl RInto<PipeValue<f32>, K>) -> &mut Self {
      let host = self.host().clone_writer();
      let mix = self.mix_builtin_widget();
      mix.init_sub_widget(height, &host, |w: &mut SizedBox, v| w.size.height = v);
      self
    }
  }

  // Test: Plain values
  let mut d = SizedBox::declarer();
  d.with_width(100.0);
  d.with_height(50.0);
  let w = d.finish();
  assert_eq!(w.read().size.width, 100.0);
  assert_eq!(w.read().size.height, 50.0);

  // Test: Pipe binding
  let width_state = Stateful::new(200.0f32);
  let height_state = Stateful::new(150.0f32);

  let mut d2 = SizedBox::declarer();
  let width_pipe = Pipe::from_watcher(width_state.clone_watcher());
  let height_pipe = Pipe::from_watcher(height_state.clone_watcher());

  d2.with_width(PipeValue::Pipe { init_value: 200.0, pipe: width_pipe });
  d2.with_height(PipeValue::Pipe { init_value: 150.0, pipe: height_pipe });

  let w2 = d2.finish();

  // Verify initial values
  assert_eq!(w2.read().size.width, 200.0);
  assert_eq!(w2.read().size.height, 150.0);

  // Update pipe sources and verify reactive updates
  *width_state.write() = 250.0;
  *height_state.write() = 180.0;

  // Pipe updates are processed in next frame
  let wnd = TestWindow::new_with_size(
    fn_widget! {
      @MockBox { size: Size::new(10., 10.) }
    },
    Size::new(100., 100.),
  );
  wnd.draw_frame();

  assert_eq!(w2.read().size.width, 250.0);
  assert_eq!(w2.read().size.height, 180.0);
}

#[test]
#[should_panic = "Required field `EagerRequired::value` not set"]
fn eager_mode_required_field_panic() {
  reset_test_env!();

  #[derive(Default)]
  #[declare(eager)]
  struct EagerRequired {
    #[allow(unused)]
    value: f32,
  }

  // Should panic because required field is not set
  let _ = EagerRequired::declarer().finish();
}
