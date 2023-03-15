use ribir::prelude::*;

fn dummy_ctx() -> &'static BuildCtx<'static> { unsafe { std::mem::transmute(&0) } }

#[test]
fn declare_builder_smoke() {
  // empty struct
  #[derive(Declare)]
  struct A;
  let _: A = ADeclarer {}.build(dummy_ctx());

  #[derive(Declare)]
  struct B {
    a: f32,
    b: i32,
  }

  let b = <B as Declare>::declare_builder()
    .a(1.)
    .b(1)
    .build(dummy_ctx());
  assert_eq!(b.a, 1.);
  assert_eq!(b.b, 1);
}

#[test]
#[should_panic = "Required field `T::_a` not set"]
fn panic_if_miss_require_field() {
  #[derive(Declare)]
  struct T {
    _a: f32,
  }

  let _ = <T as Declare>::declare_builder().build(dummy_ctx());
}

#[test]

fn empty_default_field() {
  #[derive(Declare)]
  struct T {
    #[declare(default)]
    a: f32,
  }

  let t = <T as Declare>::declare_builder().build(dummy_ctx());
  assert_eq!(t.a, 0.);
}

#[test]

fn string_default_field() {
  #[derive(Declare)]
  struct T {
    #[declare(default = "hi!")]
    text: &'static str,
  }

  let t = <T as Declare>::declare_builder().build(dummy_ctx());
  assert_eq!(t.text, "hi!");
}
