use std::{cell::RefCell, rc::Rc};

use smallvec::smallvec;

use super::providers::Setup;
use crate::prelude::*;

type ReuseFactory = Box<dyn Fn() -> Widget<'static>>;

#[derive(Default)]
struct ReuseScopeInner {
  entries: RefCell<ahash::HashMap<ReuseKey, ReuseEntry>>,
}

#[derive(Default)]
struct ReuseEntry {
  factory: Option<ReuseFactory>,
  instance: EntryInstance,
}

#[derive(Default)]
enum EntryInstance {
  #[default]
  None,
  Bound(ReuseHandle),
  LeftLive,
}

enum LookupState {
  Resolvable,
  LeftLive,
}

struct ReuseScopeSetup(ReuseScope);

struct ReuseScopeRestore {
  inner: Box<dyn ProviderRestore>,
  scope: ReuseScope,
}

fn default_reuse_scope_inner() -> Rc<ReuseScopeInner> { Rc::new(ReuseScopeInner::default()) }

/// A scope that defines a reuse boundary and owns the registry for keys
/// visible inside that boundary.
#[declare(stateless)]
pub struct ReuseScope {
  #[declare(skip, default = default_reuse_scope_inner())]
  inner: Rc<ReuseScopeInner>,
  #[declare(custom, default)]
  defs: Vec<ReuseDef>,
}

/// A lazily registered definition inside a [`ReuseScope`].
pub struct ReuseDef {
  key: ReuseKey,
  factory: ReuseFactory,
}

impl ReuseScopeInner {
  fn contains(&self, key: &ReuseKey) -> bool { self.entries.borrow().contains_key(key) }

  fn binding_count(&self) -> usize { self.entries.borrow().len() }

  fn get_keys(&self) -> impl Iterator<Item = ReuseKey> {
    self
      .entries
      .borrow()
      .keys()
      .cloned()
      .collect::<Vec<_>>()
      .into_iter()
  }

  fn register_def(&self, key: ReuseKey, factory: ReuseFactory) {
    debug_assert!(key.is_local(), "defs only support ReuseKey::local(...)");

    let mut entries = self.entries.borrow_mut();
    let entry = entries.entry(key).or_default();
    debug_assert!(
      entry.factory.is_none() && matches!(entry.instance, EntryInstance::None),
      "Duplicate local reuse definition in the same ReuseScope."
    );
    entry.factory = Some(factory);
  }

  fn lookup_state(&self, key: &ReuseKey) -> Option<LookupState> {
    self.entries.borrow().get(key).map(|entry| {
      if matches!(entry.instance, EntryInstance::LeftLive) {
        LookupState::LeftLive
      } else {
        LookupState::Resolvable
      }
    })
  }

  fn resolve<'a>(&self, key: &ReuseKey) -> Option<Widget<'a>> {
    let mut entries = self.entries.borrow_mut();
    let entry = entries.get_mut(key)?;
    match &entry.instance {
      EntryInstance::LeftLive => return None,
      EntryInstance::Bound(handle) => return Some(handle.get_widget()),
      EntryInstance::None => {}
    }

    let factory = entry.factory.as_ref()?;
    let (widget, handle) = ReuseHandle::new(factory());
    entry.instance = EntryInstance::Bound(handle);
    Some(widget)
  }

  fn insert_from_child<'a>(&self, key: ReuseKey, widget: Widget<'a>) -> Widget<'a> {
    let mut entries = self.entries.borrow_mut();
    let entry = entries.entry(key).or_default();
    debug_assert!(!matches!(entry.instance, EntryInstance::LeftLive));
    let (widget, handle) = ReuseHandle::new(widget);
    entry.instance = EntryInstance::Bound(handle);
    widget
  }

  fn assert_no_defs_conflict(&self, key: &ReuseKey) {
    if key.is_local() {
      let has_defs = self
        .entries
        .borrow()
        .get(key)
        .is_some_and(|entry| entry.factory.is_some());
      assert!(
        !has_defs,
        "ReuseScope local key cannot be defined by both defs and inline @Reuse child."
      );
    }
  }

  fn leave(&self, key: &ReuseKey) -> ReuseLeaveResult {
    let mut entries = self.entries.borrow_mut();
    let (result, should_remove) = if let Some(entry) = entries.get_mut(key) {
      match &entry.instance {
        EntryInstance::Bound(handle) => {
          if handle.is_in_use() {
            entry.instance = EntryInstance::LeftLive;
            (ReuseLeaveResult::LiveLeft, false)
          } else {
            entry.instance = EntryInstance::None;
            (ReuseLeaveResult::CachedLeft, entry.factory.is_none())
          }
        }
        _ => (ReuseLeaveResult::NotFound, false),
      }
    } else {
      (ReuseLeaveResult::NotFound, false)
    };

    if should_remove {
      entries.remove(key);
    }
    result
  }

  fn prune_detached(&self, key: &ReuseKey) {
    let mut entries = self.entries.borrow_mut();
    let should_remove = entries.get_mut(key).is_some_and(|entry| {
      if let EntryInstance::Bound(handle) = &entry.instance
        && !handle.is_in_use()
      {
        entry.instance = EntryInstance::None;
      }
      matches!(entry.instance, EntryInstance::None) && entry.factory.is_none()
    });

    if should_remove {
      entries.remove(key);
    }
  }

  fn finalize_disposed(&self, key: &ReuseKey) {
    let mut entries = self.entries.borrow_mut();
    let should_remove = entries.get_mut(key).is_some_and(|entry| {
      match &entry.instance {
        EntryInstance::Bound(handle) if handle.is_in_use() => return false,
        _ => entry.instance = EntryInstance::None,
      }
      entry.factory.is_none()
    });

    if should_remove {
      entries.remove(key);
    }
  }
}

impl ReuseScope {
  pub(crate) fn root() -> Self { Self::new() }

  fn new() -> Self { Self { inner: default_reuse_scope_inner(), defs: vec![] } }

  fn ptr_eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.inner, &other.inner) }

  pub(crate) fn root_provider() -> Provider {
    Provider::Setup(Box::new(ReuseScopeSetup(Self::root())))
  }

  fn into_provider(self) -> Provider { Provider::Setup(Box::new(ReuseScopeSetup(self))) }

  fn current_scope(ctx: &impl AsRef<ProviderCtx>) -> ReuseScope {
    Self::visible_scopes(ctx)
      .last()
      .cloned()
      .expect("Reuse should always be built inside an implicit or explicit ReuseScope")
  }

  fn root_scope(ctx: &impl AsRef<ProviderCtx>) -> ReuseScope {
    Self::visible_scopes(ctx)
      .first()
      .cloned()
      .expect("Reuse should always be built inside an implicit or explicit ReuseScope")
  }

  fn visible_scopes(ctx: &impl AsRef<ProviderCtx>) -> &[ReuseScope] {
    ctx.as_ref().visible_reuse_scopes()
  }

  fn lookup_scope_state(
    ctx: &impl AsRef<ProviderCtx>, key: &ReuseKey,
  ) -> Option<(ReuseScope, LookupState)> {
    if key.is_bound_locally() {
      let scope = Self::current_scope(ctx);
      let state = scope.inner.lookup_state(key);
      state.map(|state| (scope, state))
    } else {
      Self::visible_scopes(ctx)
        .iter()
        .rev()
        .find_map(|scope| {
          scope
            .inner
            .lookup_state(key)
            .map(|state| (scope.clone(), state))
        })
    }
  }

  pub(crate) fn resolve<'a>(
    ctx: &impl AsRef<ProviderCtx>, key: &ReuseKey,
  ) -> Option<(ReuseScope, Widget<'a>)> {
    let (scope, LookupState::Resolvable) = Self::lookup_scope_state(ctx, key)? else {
      return None;
    };
    let widget = scope.inner.resolve(key)?;
    Some((scope, widget))
  }

  pub(crate) fn resolve_or_build<'a>(
    ctx: &impl AsRef<ProviderCtx>, key: &ReuseKey, widget: Widget<'a>,
  ) -> (ReuseScope, Widget<'a>) {
    if let Some((scope, state)) = Self::lookup_scope_state(ctx, key) {
      if matches!(state, LookupState::LeftLive) {
        panic!(
          "reuse key has left its scope and cannot resolve again until the live widget disposes"
        );
      }
      scope.inner.assert_no_defs_conflict(key);
      let widget = scope
        .inner
        .resolve(key)
        .expect("reuse key should resolve from existing binding");
      return (scope, widget);
    }

    let target =
      if key.is_bound_locally() { Self::current_scope(ctx) } else { Self::root_scope(ctx) };
    let widget = target
      .inner
      .insert_from_child(key.clone(), widget);
    (target, widget)
  }

  pub(crate) fn leave(ctx: &impl AsRef<ProviderCtx>, key: &ReuseKey) -> ReuseLeaveResult {
    let Some((scope, state)) = Self::lookup_scope_state(ctx, key) else {
      return ReuseLeaveResult::NotFound;
    };
    if matches!(state, LookupState::LeftLive) {
      return ReuseLeaveResult::NotFound;
    }
    scope.inner.leave(key)
  }

  pub(crate) fn prune_detached(&self, key: &ReuseKey) { self.inner.prune_detached(key); }

  pub(crate) fn finalize_disposed(&self, key: &ReuseKey) { self.inner.finalize_disposed(key); }

  pub fn contains_binding(&self, key: &ReuseKey) -> bool { self.inner.contains(key) }

  pub fn binding_count(&self) -> usize { self.inner.binding_count() }

  pub fn keys(&self) -> impl Iterator<Item = ReuseKey> {
    self
      .inner
      .get_keys()
      .collect::<Vec<_>>()
      .into_iter()
  }

  fn register_def(&self, def: ReuseDef) { self.inner.register_def(def.key, def.factory); }

  fn install_defs(&mut self) {
    let defs = std::mem::take(&mut self.defs);
    defs
      .into_iter()
      .for_each(|def| self.register_def(def));
  }
}

impl Clone for ReuseScope {
  fn clone(&self) -> Self { Self { inner: self.inner.clone(), defs: vec![] } }
}

impl ReuseDef {
  fn new(key: ReuseKey, factory: ReuseFactory) -> Self { Self { key, factory } }
}

impl ReuseScopeDeclarer {
  pub fn with_defs(&mut self, defs: impl IntoIterator<Item = ReuseDef>) -> &mut Self {
    if let Some(stored) = self.defs.as_mut() {
      stored.extend(defs);
    } else {
      self.defs = Some(defs.into_iter().collect());
    }
    self
  }
}

impl ProviderSetup for ReuseScopeSetup {
  fn setup(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderRestore> {
    let ReuseScopeSetup(scope) = *self;
    map.push_reuse_scope(scope.clone());
    let inner = Box::new(Setup::new(scope.clone())).setup(map);
    Box::new(ReuseScopeRestore { inner, scope })
  }
}

impl ProviderRestore for ReuseScopeRestore {
  fn restore(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderSetup> {
    let ReuseScopeRestore { inner, scope } = *self;
    drop(inner.restore(map));
    let popped = map.pop_reuse_scope();
    debug_assert!(popped.is_some_and(|current| current.ptr_eq(&scope)));
    Box::new(ReuseScopeSetup(scope))
  }
}

impl<'w> ComposeChild<'w> for ReuseScope {
  type Child = Widget<'w>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'w> {
    let mut this = match this.try_into_value() {
      Ok(v) => v,
      Err(_) => panic!("ReuseScope should be a stateless widget"),
    };
    this.install_defs();

    fn_widget! {
      @Providers {
        providers: smallvec![this.clone().into_provider()],
        @ { child }
      }
    }
    .into_widget()
  }
}

pub fn reuse_def(key: ReuseKey, factory: impl Fn() -> Widget<'static> + 'static) -> ReuseDef {
  ReuseDef::new(key, Box::new(factory))
}

#[cfg(test)]
mod tests {
  use std::{
    cell::{Cell, RefCell},
    rc::Rc,
  };

  use super::*;
  use crate::test_helper::*;

  #[test]
  fn local_key() {
    reset_test_env!();
    let (build_cnt, build_w) = split_value(0);
    let (item_cnt, item_w) = split_value(1);
    let reuse_scope = Rc::new(RefCell::new(None));
    let reuse_scope2 = reuse_scope.clone();
    let w = fn_widget! {

      @ReuseScope {
        on_mounted: {
          let reuse_scope2 = reuse_scope2.clone();
          move |e| {
            *reuse_scope2.borrow_mut() = Some(Provider::of::<ReuseScope>(e).unwrap().clone());
          }
        },
        @MockMulti {
          @pipe! {
            (0..*$read(item_cnt)).map(move |i| {
              @Reuse {
                reuse: ReuseKey::local(i),
                @ {
                  fn_widget! {
                    *$write(build_w) += 1;
                    Void::default().into_widget()
                  }
                }
              }
            })
          }
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();

    let reuse_scope = reuse_scope.borrow_mut().take().unwrap();
    assert_eq!(*build_cnt.read(), 1);
    assert_eq!(reuse_scope.binding_count(), 1);

    *item_w.write() = 4;
    wnd.draw_frame();

    assert_eq!(*build_cnt.read(), 4);
    assert_eq!(reuse_scope.binding_count(), 4);

    *item_w.write() = 2;
    wnd.draw_frame();
    wnd.draw_frame();

    assert_eq!(*build_cnt.read(), 4);
    assert_eq!(reuse_scope.binding_count(), 2);
  }

  #[test]
  fn defs_enable_resolve_only() {
    reset_test_env!();

    let build_cnt = Rc::new(Cell::new(0));
    let header = ReuseKey::local("header");
    let defs_header = header.clone();
    let defs_build_cnt = build_cnt.clone();

    let wnd = TestWindow::from_widget(fn_widget! {
      @ReuseScope {
        defs: [reuse_def(defs_header.clone(), {
          let defs_build_cnt = defs_build_cnt.clone();
          move || {
            defs_build_cnt.set(defs_build_cnt.get() + 1);
            Void::default().into_widget()
          }
        })],
        @header.resolve()
      }
    });

    wnd.draw_frame();
    assert_eq!(build_cnt.get(), 1);
  }

  #[test]
  fn defs_survive_instance_removal() {
    reset_test_env!();

    let build_cnt = Rc::new(Cell::new(0));
    let show = Stateful::new(true);
    let key = ReuseKey::local("header");
    let defs_key = key.clone();
    let defs_build_cnt = build_cnt.clone();
    let scope = Rc::new(RefCell::new(None));
    let scope2 = scope.clone();

    let wnd = TestWindow::from_widget(fn_widget! {
      @ReuseScope {
        defs: [reuse_def(defs_key.clone(), {
          let defs_build_cnt = defs_build_cnt.clone();
          move || {
            defs_build_cnt.set(defs_build_cnt.get() + 1);
            Void::default().into_widget()
          }
        })],
        on_mounted: {
          let scope2 = scope2.clone();
          move |e| *scope2.borrow_mut() = Some(Provider::of::<ReuseScope>(e).unwrap().clone())
        },
        @ {
          pipe!(*$read(show)).map({
            let key = key.clone();
            move |show| show.then(|| key.resolve())
          })
        }
      }
    });

    wnd.draw_frame();
    let scope = scope.borrow().clone().unwrap();
    assert_eq!(build_cnt.get(), 1);
    assert_eq!(scope.binding_count(), 1);

    *show.write() = false;
    wnd.draw_frame();
    wnd.draw_frame();
    assert_eq!(scope.binding_count(), 1);

    *show.write() = true;
    wnd.draw_frame();
    assert_eq!(build_cnt.get(), 2);
    assert_eq!(scope.binding_count(), 1);
  }

  #[test]
  fn leave_live_binding_marks_tombstone() {
    reset_test_env!();

    let leave_result = Rc::new(RefCell::new(None::<ReuseLeaveResult>));
    let scope = Rc::new(RefCell::new(None));
    let scope2 = scope.clone();
    let key = ReuseKey::local("live");
    let leave_result2 = leave_result.clone();
    let test_key = key.clone();

    let wnd = TestWindow::from_widget(fn_widget! {
      let key = key.clone();
      let content_key = key.clone();
      let leave_result = leave_result2.clone();
      @ReuseScope {
        on_mounted: {
          let scope2 = scope2.clone();
          move |e| *scope2.borrow_mut() = Some(Provider::of::<ReuseScope>(e).unwrap().clone())
        },
        @MockMulti {
          @ {
            Some(fn_widget! {
              @Reuse {
                reuse: content_key.clone(),
                @MockBox { size: Size::zero() }
              }
            })
          }
          @MockBox {
            size: Size::zero(),
            on_performed_layout: move |e| {
              *leave_result.borrow_mut() = Some(key.leave(e));
            },
          }
        }
      }
    });

    wnd.draw_frame();
    let scope = scope.borrow().clone().unwrap();
    assert_eq!(*leave_result.borrow(), Some(ReuseLeaveResult::LiveLeft));
    assert_eq!(scope.binding_count(), 1);
    assert!(matches!(scope.inner.lookup_state(&test_key), Some(LookupState::LeftLive)));
    assert!(scope.inner.resolve(&test_key).is_none());
  }

  #[test]
  fn leave_cached_global_binding_removes_scope_entry() {
    reset_test_env!();

    let show = Stateful::new(true);
    let scope = Rc::new(RefCell::new(None));
    let scope2 = scope.clone();
    let key = ReuseKey::global("global");
    let test_key = key.clone();

    let wnd = TestWindow::from_widget(fn_widget! {
      let key = key.clone();
      @MockMulti {
        on_mounted: {
          let scope2 = scope2.clone();
          move |e| *scope2.borrow_mut() = Some(Provider::of::<ReuseScope>(e).unwrap().clone())
        },
        @ {
          pipe!(*$read(show)).map({
            let key = key.clone();
            move |show| {
              show.then({
                let key = key.clone();
                move || fn_widget! {
                  @Reuse {
                    reuse: key.clone(),
                    @MockBox { size: Size::zero() }
                  }
                }
              })
            }
          })
        }
      }
    });

    wnd.draw_frame();
    let scope = scope.borrow().clone().unwrap();
    assert_eq!(scope.binding_count(), 1);

    *show.write() = false;
    wnd.draw_frame();
    assert_eq!(scope.binding_count(), 1);

    let leave_result = scope.inner.leave(&test_key);
    assert_eq!(leave_result, ReuseLeaveResult::CachedLeft);
    assert_eq!(scope.binding_count(), 0);
  }

  #[test]
  fn nested_scopes_are_visible_in_provider_context_order() {
    reset_test_env!();

    let seen_chain = Rc::new(RefCell::new(Vec::new()));
    let nearest_scope = Rc::new(RefCell::new(None));
    let seen_chain2 = seen_chain.clone();
    let nearest_scope2 = nearest_scope.clone();

    let wnd = TestWindow::from_widget(fn_widget! {
      let seen_chain = seen_chain2.clone();
      let nearest_scope = nearest_scope2.clone();
      @ReuseScope {
        @ReuseScope {
          @fn_widget! {
            *seen_chain.borrow_mut() = ReuseScope::visible_scopes(BuildCtx::get()).to_vec();
            *nearest_scope.borrow_mut() = Some(
              Provider::of::<ReuseScope>(BuildCtx::get()).unwrap().clone(),
            );
            Void::default().into_widget()
          }
        }
      }
    });

    wnd.draw_frame();

    let seen_chain = seen_chain.borrow().clone();
    let nearest_scope = nearest_scope.borrow().clone().unwrap();

    assert_eq!(seen_chain.len(), 3);
    assert!(seen_chain[2].ptr_eq(&nearest_scope));
    assert!(!seen_chain[0].ptr_eq(&seen_chain[1]));
    assert!(!seen_chain[1].ptr_eq(&seen_chain[2]));
  }
}
