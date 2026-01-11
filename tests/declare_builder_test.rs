use ribir::{core::reset_test_env, prelude::*};

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
  #[simple_declare]
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
  let _ = s.with_margin(EdgeInsets::all(10.));
}
