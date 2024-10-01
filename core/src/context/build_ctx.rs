use std::{ptr::NonNull, rc::Rc};

use smallvec::SmallVec;
use widget_id::{new_node, RenderQueryable};

use crate::{pipe::DynInfo, prelude::*, render_helper::PureRender, window::WindowId};

/// A context provide during build the widget tree.
pub struct BuildCtx {
  /// Widgets from the root widget to the current widget provide data that the
  /// descendants can access.
  pub(crate) providers: SmallVec<[WidgetId; 1]>,
  /// Providers are available for the preallocated widget but have not been
  /// attached yet.
  pub(crate) current_providers: SmallVec<[Box<dyn Query>; 1]>,
  /// A node ID has already been allocated for the current building node.
  pub(crate) pre_alloc: Option<WidgetId>,
  pub(crate) tree: NonNull<WidgetTree>,
  // Todo: Since `Theme`, `Palette`, `TypographyTheme` and `TextStyle` are frequently queried
  // during the building process, layout and paint. we should cache the closest one.
}

/// A handle of `BuildCtx` that you can store it and access the `BuildCtx` later
/// in anywhere.
#[derive(Debug, Clone)]
pub struct BuildCtxHandle {
  startup: StartUpWidget,
  wnd_id: WindowId,
}

/// The widget from which to start creating the build context.
#[derive(Debug, Clone)]
enum StartUpWidget {
  // The static widget ID.
  Id(WidgetId),
  // The pipe node info contains the widget ID. Since widgets may be regenerated,
  // we use a lazy approach to retrieve it.
  PipeNode(DynInfo),
}

impl BuildCtx {
  /// Return the window of this context is created from.
  pub fn window(&self) -> Rc<Window> { self.tree().window() }

  /// Generate a handle for this `BuildCtx` that supports `Clone`, and
  /// can be converted back to this `BuildCtx`. This allows you to store the
  /// `BuildCtx`.
  pub fn handle(&self) -> BuildCtxHandle {
    // When the current widget is a pipe node. This widget ID may be updated in the
    // future, so we store the pipe info to ensure we always retrieve the correct
    // widget ID when using the handle.
    //
    // If the handle in the pipe subtree but not the pipe node (root), even
    // though it will be regenerated, we can store the static widget ID because the
    // handle will also be regenerated.
    let id = *self.providers.last().unwrap();
    let startup = if let Some(info) = self
      .current_providers
      .iter()
      .find_map(|p| p.query(TypeId::of::<DynInfo>()))
      .and_then(|h| h.into_ref())
      .or_else(|| id.query_ref::<DynInfo>(self.tree()))
    {
      StartUpWidget::PipeNode(info.clone())
    } else {
      StartUpWidget::Id(id)
    };

    BuildCtxHandle { wnd_id: self.window().id(), startup }
  }

  #[inline]
  pub(crate) fn create(startup: WidgetId, tree: NonNull<WidgetTree>) -> BuildCtxGuard {
    BuildCtxGuard::new(startup, tree)
  }

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
    if let Some(id) = self.pre_alloc.take() {
      *id.get_node_mut(self.tree_mut()).unwrap() = node;
      id
    } else {
      new_node(&mut self.tree_mut().arena, node)
    }
  }

  pub(crate) fn pre_alloc(&mut self) -> WidgetId {
    if let Some(id) = self.pre_alloc {
      id
    } else {
      let id = new_node(&mut self.tree_mut().arena, Box::new(PureRender(Void)));
      self.pre_alloc = Some(id);
      id
    }
  }

  pub(crate) fn consume_root_with_provider<'w>(
    &mut self, w: Widget<'w>, provider: Box<dyn Query>,
  ) -> (Widget<'w>, Box<dyn Query>) {
    self.current_providers.push(provider);
    let (w, _) = w.consume_root(self);
    let provider = self.current_providers.pop().unwrap();
    (w, provider)
  }
}

impl BuildCtxHandle {
  /// Acquires a reference to the `BuildCtx` in this handle, maybe not exist if
  /// the window is closed or widget is removed.
  ///
  /// # Panic
  ///
  /// Panics if the widget node of the handle is removed.
  pub fn with_ctx<R>(&self, f: impl FnOnce(&mut BuildCtx) -> R) -> Option<R> {
    AppCtx::get_window(self.wnd_id).map(|wnd: Rc<Window>| {
      let id = match &self.startup {
        StartUpWidget::Id(id) => *id,
        StartUpWidget::PipeNode(p) => p.borrow().host_id(),
      };
      let mut ctx = BuildCtx::create(id, wnd.tree);
      f(&mut ctx)
    })
  }
}

/// During widget building, if a subtree uses a handle to create a BuildCtx, it
/// may lack current context information because the building subtree has not
/// been added to the tree yet.
///
/// For example, when constructing subtree A, where B is a sub-tree of A, and
/// during the building of B and its widget, it utilizes
/// `BuildCtxHandle::with_handle` to retrieve the `BuildCtx`. At that moment, B
/// is not yet a sub-tree of A, resulting in the inability to access the
/// providers of its intended ancestors.
///
/// # Safety
///
/// We use the build context in a single thread and utilize it to
/// construct the widget tree in a top-first manner. There may be simultaneous
/// borrowing and mutation of the CURRENT_CTX, but this occurs during the
/// construction of different parts of the tree. By breaking the borrow checker
/// here, we achieve clearer logic.
static mut CURRENT_CTX: Option<BuildCtx> = None;

enum CtxRestore {
  None,
  Info { providers_len: usize, current_providers_len: usize },
}
pub(crate) struct BuildCtxGuard {
  ctx: &'static mut BuildCtx,
  restore: CtxRestore,
}

impl BuildCtxGuard {
  pub(crate) fn new(startup: WidgetId, tree: NonNull<WidgetTree>) -> Self {
    // Safety: The caller guarantees a valid tree structure.
    let t = unsafe { tree.as_ref() };
    let providers_list = startup.ancestors(t).filter(|id| id.queryable(t));

    if let Some(ctx) = unsafe { CURRENT_CTX.as_mut() } {
      let last = ctx.providers.last().copied();
      let providers: SmallVec<[WidgetId; 1]> = providers_list
        .take_while(|id| Some(*id) != last)
        .collect();

      let providers_len = ctx.providers.len();
      let current_providers_len = ctx.current_providers.len();
      ctx
        .providers
        .extend(providers.iter().rev().copied());

      BuildCtxGuard { ctx, restore: CtxRestore::Info { providers_len, current_providers_len } }
    } else {
      let mut providers: SmallVec<[WidgetId; 1]> = providers_list.collect();
      providers.reverse();

      unsafe {
        CURRENT_CTX =
          Some(BuildCtx { tree, providers, pre_alloc: None, current_providers: <_>::default() });

        BuildCtxGuard { ctx: CURRENT_CTX.as_mut().unwrap(), restore: CtxRestore::None }
      }
    }
  }
}

impl std::ops::Deref for BuildCtxGuard {
  type Target = BuildCtx;

  fn deref(&self) -> &Self::Target { &*self.ctx }
}

impl std::ops::DerefMut for BuildCtxGuard {
  fn deref_mut(&mut self) -> &mut Self::Target { self.ctx }
}

impl Drop for BuildCtxGuard {
  fn drop(&mut self) {
    match self.restore {
      CtxRestore::None => unsafe { CURRENT_CTX = None },
      CtxRestore::Info { providers_len, current_providers_len } => {
        assert!(self.ctx.providers.len() >= providers_len);
        assert!(self.ctx.current_providers.len() >= current_providers_len);
        self.ctx.providers.drain(providers_len..);
        self
          .ctx
          .current_providers
          .drain(current_providers_len..);
      }
    }
  }
}
