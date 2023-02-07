use rxrust::prelude::ObservableExt;

/// Is a struct contain a initialize value and an observable that should be
/// subscribed to update the value.
pub struct AssignObservable<V, S: ObservableExt<V, ()>> {
  value: V,
  observable: S,
}

impl<V, S> AssignObservable<V, S>
where
  S: ObservableExt<V, ()>,
{
  #[inline]
  pub fn new(init: V, observable: S) -> Self { Self { value: init, observable } }

  /// map the inner observable to another observable.
  pub fn map_stream<R>(self, f: impl FnOnce(S) -> R) -> AssignObservable<V, R>
  where
    R: ObservableExt<V, ()>,
  {
    let Self { value, observable } = self;
    let observable = f(observable);
    AssignObservable { value, observable }
  }

  ///  map the init value and observable to another and construct to a new
  /// InitObservable
  pub fn map<V2, S2>(self, f: impl FnOnce(V, S) -> (V2, S2)) -> AssignObservable<V2, S2>
  where
    S2: ObservableExt<V2, ()>,
  {
    let Self { value: init, observable } = self;
    let (init, observable) = f(init, observable);
    AssignObservable { value: init, observable }
  }

  #[inline]
  pub fn unzip(self) -> (V, S) { (self.value, self.observable) }
}
