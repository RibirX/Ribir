use crate::prelude::*;

/// Defines caching behavior for LocalWidget instances
///
/// Controls how long widgets should be retained in memory after they become
/// unused.
#[derive(Default, PartialEq, Eq, Copy, Clone, Debug)]
pub enum CachePolicy {
  /// Widgets are immediately released when no longer used
  #[default]
  ImmediateRelease,
  /// Widgets persist in cache until manually removed
  ManualControl,
}

/// ReuseId is an identifier used to recognize identical widgets in either
/// global or local scope.
///
/// If two widgets share the same `ReuseId` within their scope, they will be
/// treated as identical. During widget updates, the framework will reuse
/// existing widgets with matching `ReuseId` instead of creating new ones.
///
/// This enables:
/// - Tracking persistent widgets across frames
/// - Performance optimization through widget reuse
///
/// Important considerations:
/// - The framework relies entirely on `ReuseId`, not widget types
/// - Developers must ensure:
///   1. `ReuseId` uniqueness within its scope
///   2. Consistency between `ReuseId` and actual widget type
///
/// ReuseId variants:
/// - `GlobalId`: Window-wide reuse, requires explicit removal
/// - `LocalId`: Scoped reuse within nearest `LocalWidgets`, with caching
///   controlled by `CachePolicy` (default: [`CachePolicy::ImmediateRelease`])
///
/// # Examples
///
/// ## Local Scoped Reuse
/// ```rust
/// use ribir::prelude::*;
///
/// let widget = fn_widget! {
///   let cnt = Stateful::new(0);
///   @LocalWidgets {
///     @Column {
///       @FilledButton {
///         on_tap: move |_| *$cnt.write() += 1,
///         @ { "Increment" }
///       }
///       @ {
///         pipe!(*$cnt).map(move |cnt| move || {
///           @ {
///             (0..cnt).map(move |i| @FatObj {
///               reuse_id: LocalId::number(i),
///               @text! {
///                 text: format!("Item {i}, created at {:?}", Instant::now())
///               }
///             })
///           }
///         })
///       }
///     }
///   }
/// };
/// ```
///
/// ## Global reusable widget
/// ```rust
/// use ribir::prelude::*;
///
/// fn global_widget() -> Widget<'static> {
///   fat_obj! {
///     reuse_id: GlobalId::new("global_widget"),
///     @text! { text: "Globally reusable widget" }
///   }
///   .into_widget()
/// }
/// ```
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ReuseId {
  Global(GlobalId),
  Local(LocalId, CachePolicy),
}

/// Local-scoped widget identifier with multiple key formats
#[derive(PartialEq, Hash, Eq, Clone, Debug)]
pub enum LocalId {
  Number(usize),
  String(CowArc<str>),
}

/// Global-scoped widget identifier with string-based key
#[derive(PartialEq, Hash, Eq, Clone, Debug)]
pub struct GlobalId(CowArc<str>);

/// Widget reuse manager with automatic instance tracking
///
/// Implements a reuse strategy that:
/// 1. Checks for existing instances matching the `ReuseId`
/// 2. Reuses found instances or registers new ones
/// 3. Applies cache policies for automatic cleanup
///
/// Registration scope depends on `ReuseId` type:
/// - `GlobalId`: Registered in global widget registry
/// - `LocalId`: Registered in nearest local widget scope
#[derive(Clone)]
pub struct Reuse {
  pub reuse_id: ReuseId,
}

impl GlobalId {
  /// Creates a new global ID from a convertible key type
  pub fn new(key: impl Into<CowArc<str>>) -> Self { GlobalId(key.into()) }
}

impl LocalId {
  /// Creates a text-based local ID
  pub fn string(key: impl Into<CowArc<str>>) -> Self { LocalId::String(key.into()) }

  /// Creates a numeric local ID
  pub fn number(key: usize) -> Self { LocalId::Number(key) }

  /// Converts to ReuseId with specified cache policy
  pub fn with_policy(self, policy: CachePolicy) -> ReuseId { ReuseId::Local(self, policy) }
}

impl From<LocalId> for ReuseId {
  /// Creates a ReuseId with default cache policy
  fn from(value: LocalId) -> Self { ReuseId::Local(value, CachePolicy::default()) }
}

impl From<GlobalId> for ReuseId {
  /// Creates a global-scoped ReuseId
  fn from(value: GlobalId) -> Self { ReuseId::Global(value) }
}

impl<T: Into<CowArc<str>>> From<T> for GlobalId {
  /// Converts string-like types to GlobalId
  fn from(value: T) -> Self { GlobalId(value.into()) }
}

impl ReuseId {
  /// Returns true for local-scoped identifiers
  pub fn is_local(&self) -> bool { matches!(self, ReuseId::Local(..)) }

  /// Returns true for global-scoped identifiers
  pub fn is_global(&self) -> bool { matches!(self, ReuseId::Global(..)) }
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
