use ribir::prelude::*;

fn dummy_ctx() -> &'static BuildCtx<'static> { unsafe { std::mem::transmute(&0) } }

#[test]
fn declare_builder_smoke() {
  // empty struct
  #[derive(Declare)]
  struct A;
  let _: A = ADeclarer {}.build_declare(dummy_ctx());

  #[derive(Declare)]
  struct B {
    a: f32,
    b: i32,
  }

  let b = <B as Declare>::declare_builder()
    .a(1.)
    .b(1)
    .build_declare(dummy_ctx());
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

  let _ = <T as Declare>::declare_builder().build_declare(dummy_ctx());
}

#[test]

fn default_field() {
  #[derive(Declare)]
  struct DefaultDeclare {
    #[declare(default)]
    a: f32,
  }

  let t = <DefaultDeclare as Declare>::declare_builder().build_declare(dummy_ctx());
  assert_eq!(t.a, 0.);
}

#[test]

fn default_field_with_value() {
  #[derive(Declare)]
  struct DefaultWithValue {
    #[declare(default = "hi!")]
    text: &'static str,
  }

  let t = <DefaultWithValue as Declare>::declare_builder().build_declare(dummy_ctx());
  assert_eq!(t.text, "hi!");
}
