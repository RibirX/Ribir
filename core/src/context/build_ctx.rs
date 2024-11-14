#![allow(static_mut_refs)]
use std::ptr::NonNull;

use smallvec::SmallVec;
use widget_id::{RenderQueryable, new_node};

use crate::{local_sender::LocalSender, prelude::*};

/// A context provide during build the widget tree.
pub struct BuildCtx {
  /// Widgets from the root widget to the current widget provide data that the
  /// descendants can access.
  pub(crate) providers: SmallVec<[WidgetId; 1]>,
  /// Providers are available for the preallocated widget but have not been
  /// attached yet.
  pub(crate) current_providers: SmallVec<[Box<dyn Query>; 1]>,
  pub(crate) tree: NonNull<WidgetTree>,
  // Todo: Since `Theme`, `Palette`, `TypographyTheme` and `TextStyle` are frequently queried
  // during the building process, layout and paint. we should cache the closest one.
}

impl BuildCtx {
  /// Return the window of this context is created from.
  pub fn window(&self) -> Sc<Window> { self.tree().window() }

  pub(crate) fn tree(&self) -> &WidgetTree {
    // Safety: Please refer to the comments in `WidgetTree::tree_mut` for more
    // information.
    unsafe { self.tree.as_ref() }
  }

  pub(crate) fn tree_mut(&mut self) -> &mut WidgetTree {
    let mut tree = self.tree;
    // Safety:
    // The widget tree is only used for building the widget tree. Even if there are
    // multiple mutable references, they are only involved in constructing specific
    // parts of the tree.
    unsafe { tree.as_mut() }
  }

  pub(crate) fn alloc(&mut self, node: Box<dyn RenderQueryable>) -> WidgetId {
    new_node(&mut self.tree_mut().arena, node)
  }
}

/// The global variable that stores the build context is only accessible within
/// a single thread. Accessing it from another thread will result in a panic.
///
/// Before the building phase starts, the framework initializes it, and the
/// `BuildCtx` can be accessed from anywhere by calling `BuildCtx::get`. After
/// the build phase is finished, it will be set to `None`.
static mut CTX: Option<LocalSender<BuildCtx>> = None;

impl BuildCtx {
  /// Return the build context if the caller is currently in the building
  /// process.
  pub fn try_get() -> Option<&'static BuildCtx> { unsafe { CTX.as_deref() } }

  /// Return the context of the current build process. If the caller is not in
  /// the build process, a panic will occur.
  pub fn get() -> &'static BuildCtx {
    BuildCtx::try_get().expect("Not during the widget building process.")
  }

  /// Return the mutable context of the current build process. If the caller is
  /// not in the build process, a panic will occur.
  pub(crate) fn get_mut() -> &'static mut BuildCtx {
    unsafe {
      CTX
        .as_deref_mut()
        .expect("Not during the widget building process.")
    }
  }

  #[must_use]
  pub(crate) fn init_for(startup: WidgetId, tree: NonNull<WidgetTree>) -> BuildCtxInitdGuard {
    Self::set_for(startup, tree);
    BuildCtxInitdGuard
  }

  #[must_use]
  pub(crate) fn init(ctx: BuildCtx) -> BuildCtxInitdGuard {
    BuildCtx::set(ctx);
    BuildCtxInitdGuard
  }

  pub(crate) fn set_for(startup: WidgetId, tree: NonNull<WidgetTree>) {
    let t = unsafe { tree.as_ref() };
    let mut providers: SmallVec<[WidgetId; 1]> = startup
      .ancestors(t)
      .filter(|id| id.queryable(t))
      .collect();
    providers.reverse();
    let ctx = BuildCtx { tree, providers, current_providers: <_>::default() };
    BuildCtx::set(ctx);
  }

  pub(crate) fn set(ctx: BuildCtx) { unsafe { CTX = Some(LocalSender::new(ctx)) } }

  pub(crate) fn clear() { unsafe { CTX = None } }
}

pub(crate) struct BuildCtxInitdGuard;

impl Drop for BuildCtxInitdGuard {
  fn drop(&mut self) { BuildCtx::clear() }
}
