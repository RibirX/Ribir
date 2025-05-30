use ribir::prelude::*;

#[test]
fn declarer_smoke() {
  // empty struct
  #[derive(Declare)]
  struct A;

  let _: FatObj<A> = A::declarer().finish();

  #[derive(Declare)]
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
#[should_panic = "Required field `T::a` not set"]
fn panic_if_miss_require_field() {
  #[derive(Declare)]
  struct T {
    a: f32,
  }

  let _ = <T as Declare>::declarer().finish();
}

#[test]

fn default_field() {
  #[derive(Declare)]
  struct DefaultDeclare {
    #[declare(default)]
    a: f32,
  }

  let t = <DefaultDeclare as Declare>::declarer().finish();
  assert_eq!(t.read().a, 0.);
}

#[test]

fn default_field_with_value() {
  #[derive(Declare)]
  struct DefaultWithValue {
    #[declare(default = "hi!")]
    text: &'static str,
  }

  let t = <DefaultWithValue as Declare>::declarer().finish();
  assert_eq!(t.read().text, "hi!");
}

#[test]
fn declarer_simple_attr() {
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
