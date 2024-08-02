use ribir::prelude::*;

fn dummy_ctx() -> &'static BuildCtx { unsafe { std::mem::transmute(&0) } }

#[test]
fn declarer_smoke() {
  // empty struct
  #[derive(Declare)]
  struct A;

  let _: FatObj<A> = A::declarer().finish(dummy_ctx());

  #[derive(Declare)]
  struct B {
    a: f32,
    b: i32,
  }

  let b = <B as Declare>::declarer()
    .a(1.)
    .b(1)
    .finish(dummy_ctx());
  assert_eq!(b.read().a, 1.);
  assert_eq!(b.read().b, 1);
}

#[test]
#[should_panic = "Required field `T::_a` not set"]
fn panic_if_miss_require_field() {
  #[derive(Declare)]
  struct T {
    _a: f32,
  }

  let _ = <T as Declare>::declarer().finish(dummy_ctx());
}

#[test]

fn default_field() {
  #[derive(Declare)]
  struct DefaultDeclare {
    #[declare(default)]
    a: f32,
  }

  let t = <DefaultDeclare as Declare>::declarer().finish(dummy_ctx());
  assert_eq!(t.read().a, 0.);
}

#[test]

fn default_field_with_value() {
  #[derive(Declare)]
  struct DefaultWithValue {
    #[declare(default = "hi!")]
    text: &'static str,
  }

  let t = <DefaultWithValue as Declare>::declarer().finish(dummy_ctx());
  assert_eq!(t.read().text, "hi!");
}

#[test]
fn declarer_simple_attr() {
  #[simple_declare]
  struct Simple {
    a: f32,
    b: i32,
  }

  let s = Simple::declarer().a(1.).b(1).finish(dummy_ctx());
  assert_eq!(s.read().a, 1.);
  assert_eq!(s.read().b, 1);
}
