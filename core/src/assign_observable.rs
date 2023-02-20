use std::convert::Infallible;

use rxrust::{ops::map::MapOp, prelude::ObservableExt};

/// Is a struct contain a initialize value and an observable that should be
/// subscribed to update the value.
pub struct AssignObservable<V, S: ObservableExt<V, Infallible>> {
  value: V,
  observable: S,
}

impl<V, S> AssignObservable<V, S>
where
  S: ObservableExt<V, Infallible>,
{
  #[inline]
  pub fn new(init: V, observable: S) -> Self { Self { value: init, observable } }

  /// map the inner observable stream to another observable that emit same type
  /// value.
  pub fn stream_map<R>(self, f: impl FnOnce(S) -> R) -> AssignObservable<V, R>
  where
    R: ObservableExt<V, Infallible>,
  {
    let Self { value, observable } = self;
    let observable = f(observable);
    AssignObservable { value, observable }
  }

  /// Creates a new `AssignObservable` which calls a closure on each element and
  /// uses its return as the value.
  pub fn map<R, F>(self, mut f: F) -> AssignObservable<R, MapOp<S, F, V>>
  where
    F: FnMut(V) -> R,
  {
    let Self { value, observable } = self;
    AssignObservable {
      value: f(value),
      observable: observable.map(f),
    }
  }

  #[inline]
  pub fn unzip(self) -> (V, S) { (self.value, self.observable) }
}
