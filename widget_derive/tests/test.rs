#![feature(trivial_bounds)]

use holiday::prelude::*;

#[derive(Widget)]
struct Test {}

#[stateful]
#[derive(Widget)]
struct TestState {
  #[state]
  a: f32,
  #[state]
  b: f32,
}

#[test]
fn derive_stateful() {
  let mut state = TestState { a: 1., b: 2. }.into_stateful();
  let mut s_ref = state.ref_cell();
  state.state_a().subscribe(move |t| {
    s_ref.borrow_mut().b = t.after;
  });
  {
    state.as_mut().a = 2.;
  }
  let state_ref = state.as_ref();
  assert_eq!(state_ref.b, 2.)
}

#[test]
fn state_derive_tuple_support() {
  #[stateful]
  #[derive(Widget)]
  struct StateTupleSupport(#[state] i32);
}

#[test]
fn derive_widget() {
  let a = Test {}.with_key("a");

  debug_assert!(a.as_render().is_none());
  debug_assert!(a.as_combination().is_none());
}

#[test]
fn ui() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/*fail.rs");
  t.pass("tests/ui/*pass.rs");
}

#[test]
fn stateful_as_render_check() {
  let w = SizedBox::empty_box(Size::zero()).into_stateful();

  assert!(w.as_render().is_some());
}
