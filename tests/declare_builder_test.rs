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
