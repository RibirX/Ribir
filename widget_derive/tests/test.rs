#![feature(
  trivial_bounds,
  proc_macro_hygiene,
  stmt_expr_attributes,
  negative_impls
)]
use ribir::prelude::*;

#[stateful]
struct TestState {
  a: f32,
  b: f32,
}

impl CombinationWidget for TestState {
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget { unreachable!() }
}

#[test]
fn derive_stateful() {
  let state = TestState { a: 1., b: 2. }.into_stateful();
  let s_ref = state.ref_cell();
  state.state_change(|s| s.a).subscribe(move |t| {
    s_ref.borrow_mut().b = t.after;
  });
  {
    state.borrow_mut().a = 2.;
  }
  let state_ref = state.borrow();
  assert_eq!(state_ref.b, 2.)
}

#[test]
fn state_derive_tuple_support() {
  #[stateful]
  struct StateTupleSupport(i32);

  impl CombinationWidget for StateTupleSupport {
    fn build(&self, _: &mut BuildCtx) -> BoxedWidget { unreachable!() }
  }
}

#[test]
fn ui() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/**/*fail.rs");
  t.pass("tests/ui/**/*pass.rs");
}

#[test]
fn stateful_as_render_check() {
  let w = SizedBox::from_size(Size::zero()).into_stateful().box_it();
  assert!(matches!(w, BoxedWidget::Render(_)));
}
