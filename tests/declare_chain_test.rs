use ribir::{core::reset_test_env, prelude::*};

#[test]
fn simple_event() {
  reset_test_env!();
  #[declare]
  struct S {
    #[declare(strict, event = on_change)]
    a: f32,
  }
  impl S {
     fn on_change<E: 'static>(&mut self, _h: impl FnMut(&mut CustomEvent<E>) + 'static) {}
  }
  
  let _ = S::declarer().finish();
}

#[test]
fn map_event() {
  reset_test_env!();
  #[declare]
  struct M {
    #[declare(strict, event = on_change.map(|e| e.data().clone() as f32))]
    a: f32,
  }
  impl M {
     // Emits i32
     fn on_change<E: 'static>(&mut self, _h: impl FnMut(&mut CustomEvent<E>) + 'static) {}
  }

  let mut m = M::declarer();
  m.on_change(|_: &mut CustomEvent<i32>| {}); // Verify generic inference
  m.with_a(1.0);
}

#[test]
fn filter_map_event() {
  reset_test_env!();
  #[declare]
  struct FM {
    #[declare(strict, event = on_change.filter(|e| *e.data() > 0).map(|e| *e.data() as f32))]
    a: f32,
  }
  impl FM {
     fn on_change<E: 'static>(&mut self, _h: impl FnMut(&mut CustomEvent<E>) + 'static) {}
  }
  let _ = FM::declarer().finish();
}
