use ribir::prelude::*;

fn dummy_ctx() -> BuildCtx<'static> {
  let ctx = std::mem::MaybeUninit::uninit();
  unsafe { ctx.assume_init() }
}
#[test]
fn declare_builder_smoke() {
  let mut ctx = dummy_ctx();
  // empty struct
  #[derive(Declare)]
  struct A;
  let _: A = ABuilder {}.build(&mut ctx);

  #[derive(Declare)]
  struct B {
    a: f32,
    b: i32,
  }

  let mut b = <B as Declare>::builder();
  b.a(1.).b(1);
  let b = b.build(&mut ctx);
  assert_eq!(b.a, 1.);
  assert_eq!(b.b, 1);

  std::mem::forget(ctx);
}

#[test]
#[should_panic = "Required field `T::_a` not set"]
fn panic_if_miss_require_field() {
  #[derive(Declare)]
  struct T {
    _a: f32,
  }

  let mut ctx = dummy_ctx();
  let _ = <T as Declare>::builder().build(&mut ctx);
  std::mem::forget(ctx);
}

#[test]

fn empty_default_field() {
  #[derive(Declare)]
  struct T {
    #[declare(default)]
    a: f32,
  }

  let mut ctx = dummy_ctx();
  let t = <T as Declare>::builder().build(&mut ctx);
  assert_eq!(t.a, 0.);
  std::mem::forget(ctx);
}

#[test]

fn string_default_field() {
  #[derive(Declare)]
  struct T {
    #[declare(default = "\"hi!\"")]
    text: &'static str,
  }

  let mut ctx = dummy_ctx();
  let t = <T as Declare>::builder().build(&mut ctx);
  assert_eq!(t.text, "hi!");

  std::mem::forget(ctx);
}
