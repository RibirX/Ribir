use ribir::prelude::*;

fn dummy_ctx() -> &'static BuildCtx<'static> { unsafe { std::mem::transmute(&0) } }

#[test]
fn declare_builder_smoke() {
  // empty struct
  #[derive(Declare2)]
  struct A;

  let _: A = A::declare2_builder().build_declare(dummy_ctx());

  #[derive(Declare2)]
  struct B {
    a: f32,
    b: i32,
  }

  let b = <B as Declare2>::declare2_builder()
    .a(1.)
    .b(1)
    .build_declare(dummy_ctx());
  assert_eq!(b.read().a, 1.);
  assert_eq!(b.read().b, 1);
}

#[test]
#[should_panic = "Required field `T::_a` not set"]
fn panic_if_miss_require_field() {
  #[derive(Declare2)]
  struct T {
    _a: f32,
  }

  let _ = <T as Declare2>::declare2_builder().build_declare(dummy_ctx());
}

#[test]

fn default_field() {
  #[derive(Declare2)]
  struct DefaultDeclare {
    #[declare(default)]
    a: f32,
  }

  let t = <DefaultDeclare as Declare2>::declare2_builder().build_declare(dummy_ctx());
  assert_eq!(t.read().a, 0.);
}

#[test]

fn default_field_with_value() {
  #[derive(Declare2)]
  struct DefaultWithValue {
    #[declare(default = "hi!")]
    text: &'static str,
  }

  let t = <DefaultWithValue as Declare2>::declare2_builder().build_declare(dummy_ctx());
  assert_eq!(t.read().text, "hi!");
}
