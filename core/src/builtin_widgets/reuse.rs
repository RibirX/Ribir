use crate::prelude::*;

/// A unified reuse key that combines identity and lookup policy.
///
/// # Examples
///
/// ## Local Scoped Reuse
/// ```rust
/// use ribir::prelude::*;
///
/// let widget = fn_widget! {
///   let cnt = Stateful::new(0);
///   @ReuseScope {
///     @Column {
///       @FilledButton {
///         on_tap: move |_| *$write(cnt) += 1,
///         @ { "Increment" }
///       }
///       @ {
///         pipe!(*$read(cnt)).map(move |cnt| {
///           (0..cnt).map(move |i| @FatObj {
///             reuse: ReuseKey::local(i),
///             @text! {
///               text: format!("Item {i}, created at {:?}", Instant::now())
///             }
///           })
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
///     reuse: ReuseKey::global("global_widget"),
///     @text! { text: "Globally reusable widget" }
///   }
///   .into_widget()
/// }
/// ```
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
enum ReuseBindingKey {
  Global(CowArc<str>),
  LocalNumber(usize),
  LocalString(CowArc<str>),
}

mod sealed {
  pub trait Sealed {}

  impl Sealed for usize {}
  impl Sealed for &str {}
  impl Sealed for String {}
  impl Sealed for crate::prelude::CowArc<str> {}
}

#[doc(hidden)]
pub trait ReuseLocalKey: sealed::Sealed {
  #[doc(hidden)]
  fn into_local_reuse_key(self) -> Result<usize, CowArc<str>>;
}

impl ReuseLocalKey for usize {
  fn into_local_reuse_key(self) -> Result<usize, CowArc<str>> { Ok(self) }
}

impl ReuseLocalKey for &str {
  fn into_local_reuse_key(self) -> Result<usize, CowArc<str>> { Err(self.to_owned().into()) }
}

impl ReuseLocalKey for String {
  fn into_local_reuse_key(self) -> Result<usize, CowArc<str>> { Err(self.into()) }
}

impl ReuseLocalKey for CowArc<str> {
  fn into_local_reuse_key(self) -> Result<usize, CowArc<str>> { Err(self) }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ReuseKey {
  binding: ReuseBindingKey,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReuseLeaveResult {
  /// No binding currently resolves from the given context.
  NotFound,
  /// A live widget left the scope's resolution surface and will age out
  /// through its normal disposal path.
  LiveLeft,
  /// A cached binding was removed from the scope immediately.
  CachedLeft,
}

/// Widget reuse manager with automatic instance tracking
///
/// Implements a reuse strategy that:
/// 1. Checks for existing instances or defs matching the `ReuseKey`
/// 2. Reuses found instances or registers new ones
/// 3. Cleans up scope-owned bindings when instances leave the tree
#[derive(Clone)]
pub struct Reuse {
  pub reuse: ReuseKey,
}

impl ReuseKey {
  /// Creates a global reuse key.
  ///
  /// Global keys resolve from the current `ReuseScope` outward to the root
  /// scope. On first miss they register in the root scope.
  pub fn global(key: impl Into<CowArc<str>>) -> Self {
    Self { binding: ReuseBindingKey::Global(key.into()) }
  }

  /// Creates a local reuse key.
  ///
  /// Local keys resolve only within the nearest visible `ReuseScope`.
  pub fn local(key: impl ReuseLocalKey) -> Self {
    let binding = match key.into_local_reuse_key() {
      Ok(key) => ReuseBindingKey::LocalNumber(key),
      Err(key) => ReuseBindingKey::LocalString(key),
    };
    Self { binding }
  }

  pub fn is_local(&self) -> bool {
    matches!(self.binding, ReuseBindingKey::LocalNumber(_) | ReuseBindingKey::LocalString(_))
  }

  pub fn is_global(&self) -> bool { matches!(self.binding, ReuseBindingKey::Global(_)) }

  /// Creates a resolve-only `Reuse` expression for this key.
  ///
  /// This is syntax sugar for:
  /// `@Reuse { reuse: key }`
  ///
  /// It does not create a `GenWidget` and does not imply building a fresh
  /// widget instance.
  pub fn resolve(&self) -> Reuse { Reuse { reuse: self.clone() } }

  /// Evicts the binding currently resolved by this key in the given provider
  /// context.
  ///
  /// This is a scope-level operation:
  /// - live targets stop participating in future resolve lookups immediately
  /// - cached targets are removed from the owning scope immediately
  ///
  /// The lookup target follows the same local/global rules as
  /// [`resolve`](Self::resolve).
  pub fn leave(&self, ctx: &impl AsRef<ProviderCtx>) -> ReuseLeaveResult {
    ReuseScope::leave(ctx, self)
  }

  pub(crate) fn is_bound_locally(&self) -> bool { self.is_local() }
}

impl Declare for Reuse {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'a> ComposeChild<'a> for Reuse {
  type Child = Widget<'a>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'a> {
    let this = this
      .try_into_value()
      .unwrap_or_else(|_| panic!("Reuse should be a stateless widget"));
    fn_widget! {
      let (scope, w) = ReuseScope::resolve_or_build(BuildCtx::get(), &this.reuse, child);
      wrap_reuse_lifecycle(this.reuse.clone(), scope, w, this.reuse.is_local())
    }
    .into_widget()
  }
}

impl Compose for Reuse {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    let this = this
      .try_into_value()
      .unwrap_or_else(|_| panic!("Reuse should be a stateless widget"));
    fn_widget! {
      let (scope, w) = ReuseScope::resolve(BuildCtx::get(), &this.reuse)
        .expect("{this.reuse:?} is not find");
      wrap_reuse_lifecycle(this.reuse.clone(), scope, w, this.reuse.is_local())
    }
    .into_widget()
  }
}

fn wrap_reuse_lifecycle<'a>(
  key: ReuseKey, scope: ReuseScope, w: Widget<'a>, prune_when_detached: bool,
) -> Widget<'a> {
  let mut w = FatObj::new(w);
  if prune_when_detached {
    let scope = scope.clone();
    let key = key.clone();
    w.on_disposing(move |_| {
      AppCtx::spawn_local(async move {
        scope.prune_detached(&key);
      });
    });
  }
  w.on_disposed(move |_| {
    AppCtx::spawn_local(async move {
      scope.finalize_disposed(&key);
    });
  });
  w.into_widget()
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use super::*;
  use crate::test_helper::*;

  #[test]
  fn local_unused_instance_is_removed_after_recycle() {
    reset_test_env!();

    let (item_cnt, item_w) = split_value(1);
    let local_scope = Rc::new(RefCell::new(None));
    let local_scope2 = local_scope.clone();

    let wnd = TestWindow::from_widget(fn_widget! {
      @ReuseScope {
        on_mounted: {
          let local_scope2 = local_scope2.clone();
          move |e| {
            *local_scope2.borrow_mut() = Some(Provider::of::<ReuseScope>(e).unwrap().clone());
          }
        },
        @MockMulti {
          @pipe! {
            (0..*$read(item_cnt)).map(move |i| {
              @Reuse {
                reuse: ReuseKey::local(i),
                @ { Void::default() }
              }
            })
          }
        }
      }
    });

    wnd.draw_frame();
    let local_scope = local_scope.borrow_mut().take().unwrap();
    assert_eq!(local_scope.binding_count(), 1);

    *item_w.write() = 4;
    wnd.draw_frame();
    assert_eq!(local_scope.binding_count(), 4);

    *item_w.write() = 2;
    wnd.draw_frame();
    wnd.draw_frame();
    assert_eq!(local_scope.binding_count(), 2);
  }
}
