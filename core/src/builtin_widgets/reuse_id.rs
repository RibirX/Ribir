use crate::prelude::*;

/// DisposePolicy
///
/// Defines the disposal policy for LocalWidget instances.
#[derive(Default, PartialEq, Eq, Copy, Clone, Debug)]
pub enum CachePolicy {
  /// The widget will be disposed (released) as soon as it is no longer in use.
  #[default]
  ImmediateRelease,

  /// The widget will remain cached until explicitly removed by the user.
  ManualControl,
}

/// ReuseId
///
/// An identifier used to reference reusable widgets in either global or local
/// scope.
///
/// - `Global`: The widget will be stored in `GlobalWidgets` and can be accessed
///   across the entire window. It must be explicitly removed when no longer
///   needed.
/// - `Local`: The widget will be stored in `LocalWidgets` with automatic
///   disposal based on the specified `DisposePolicy` (defaults to
///   `DisposePolicy::NotUsed`).
///
/// Usage:
/// - Create with `GlobalId::new()` or `LocalId::from_num()/from_str()`
/// - Convert to `ReuseId` using `.into()` or `LocalId::reuse_with_policy()`
/// - Use as `reuse_id` field in `Reuse` Widget
///
/// # Example
/// ``` no_run
/// use ribir::prelude::*;
/// let w = fn_widget! {
///  let cnt = Stateful::new(0);
///  @ LocalWidgets {
///    @Column {
///      @ FilledButton {
///        on_tap: move |_| *$cnt.write() += 1,
///        @ { "add" }
///      }
///      @ {
///        pipe!(*$cnt).map(move |cnt|
///          move || {
///            @ {
///              (0..cnt).map(move |i| {
///                 @Text {
///                  reuse_id: LocalId::from_num(i),
///                  text: format!("Item {},  create_at {:?}", i, Instant::now())
///                 }
///              })
///            }
///           }
///          )
///      }
///    }
///  }
/// };
/// App::run(w);
/// ```
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ReuseId {
  Global(GlobalId),
  Local(LocalId, CachePolicy),
}

#[derive(PartialEq, Hash, Eq, Clone, Debug)]
pub enum LocalId {
  Number(usize),
  String(CowArc<str>),
}

#[derive(PartialEq, Hash, Eq, Clone, Debug)]
pub struct GlobalId(CowArc<str>);

impl From<LocalId> for ReuseId {
  fn from(value: LocalId) -> Self { ReuseId::Local(value, CachePolicy::default()) }
}

impl From<GlobalId> for ReuseId {
  fn from(value: GlobalId) -> Self { ReuseId::Global(value) }
}

impl<T: Into<CowArc<str>>> From<T> for GlobalId {
  fn from(value: T) -> Self { GlobalId(value.into()) }
}

impl GlobalId {
  pub fn new(key: impl Into<CowArc<str>>) -> Self { GlobalId(key.into()) }
}

impl LocalId {
  pub fn from_string(key: impl Into<CowArc<str>>) -> Self { LocalId::String(key.into()) }

  pub fn from_num(key: usize) -> Self { LocalId::Number(key) }

  pub fn reuse_with_policy(self, policy: CachePolicy) -> ReuseId { ReuseId::Local(self, policy) }
}

impl ReuseId {
  pub fn is_local(&self) -> bool { matches!(self, ReuseId::Local(..)) }

  pub fn is_global(&self) -> bool { matches!(self, ReuseId::Global(..)) }
}

/// Reuse Widget, implement for builtin `reuse_id`
///
/// This Widget will directly display the corresponding widget based on the
/// ReuseId:
/// - If a widget with the corresponding ReuseId can be found, it will directly
///   display that widget.
/// - If a widget with the corresponding ReuseId cannot be found, it will
///   display its child widget and register it in GlobalWidgets or LocalWidgets
///   according to the type of ReuseId (Global or Local) for future reference.
#[derive(Clone)]
pub struct Reuse {
  pub reuse_id: ReuseId,
}

impl Declare for Reuse {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'a> ComposeChild<'a> for Reuse {
  type Child = Widget<'a>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'a> {
    let this = match this.try_into_value() {
      Ok(v) => v,
      Err(_) => {
        panic!("Reuse should be a stateless widget");
      }
    };
    fn_widget! {
      match this.reuse_id {
        ReuseId::Global(key) => {
          let p = Provider::state_of
            ::<Box<dyn StateWriter<Value = GlobalWidgets>>>(BuildCtx::get())
            .unwrap();
          get_or_insert(&*p, &key, child).expect("{this.reuse_id:?} is not find")
        },
        ReuseId::Local(key, policy) => {
          let p = Provider::state_of
            ::<Box<dyn StateWriter<Value = LocalWidgets>>>(BuildCtx::get())
            .unwrap();
          let w = get_or_insert(&*p, &key, child).expect("{this.reuse_id:?} is not find");
          if policy == CachePolicy::ImmediateRelease {
            wrap_dispose_recycled(&key, &*p, w)
          } else {
            w
          }
        },
      }
    }
    .into_widget()
  }
}

impl Compose for Reuse {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    let this = match this.try_into_value() {
      Ok(v) => v,
      Err(_) => {
        panic!("Reuse should be a stateless widget");
      }
    };
    fn_widget! {
      match this.reuse_id {
        ReuseId::Global(key) => {
          let p = Provider::of::<GlobalWidgets>(BuildCtx::get())
            .unwrap();
          p.get(&key).expect("{this.reuse_id:?} is not find")
        },
        ReuseId::Local(key, policy) => {
          let p = Provider::state_of
            ::<Box<dyn StateWriter<Value = LocalWidgets>>>(BuildCtx::get())
            .unwrap();
          let w = $p.get(&key).expect("{this.reuse_id:?} is not find");
          if policy == CachePolicy::ImmediateRelease {
            wrap_dispose_recycled(&key, &*p, w)
          } else {
            w
          }
        },
      }
    }
    .into_widget()
  }
}

fn wrap_dispose_recycled<'a>(
  id: &LocalId, scope: &impl StateWriter<Value = LocalWidgets>, w: Widget<'a>,
) -> Widget<'a> {
  let mut w = FatObj::new(w);
  let p = scope.clone_writer();
  let key = id.clone();
  w.on_disposed(move |e| {
    let _ = e.window().frame_spawn(async move {
      if !p.read().is_in_used(&key) {
        p.write().remove(&key);
      }
    });
  });
  w.into_widget()
}
