#![allow(static_mut_refs)]
use std::ptr::NonNull;

use crate::{local_sender::LocalSender, prelude::*};

/// A context provide during build the widget tree.
pub struct BuildCtx {
  tree: NonNull<WidgetTree>,
  provider_ctx: ProviderCtx,
  /// The stack is used to temporarily store the children's relationships
  /// during the build process. The widget's lifetime remains valid throughout
  /// this process; hence, we use 'static to avoid introducing a lifetime for
  /// the BuildCtx.
  children: Vec<(WidgetId, Widget<'static>)>,
}

impl BuildCtx {
  /// Return the window of this context is created from.
  pub fn window(&self) -> Sc<Window> { self.tree().window() }

  /// Return the variant of `Color` provided in the current build context.
  pub fn color() -> Variant<Color> { Variant::new(BuildCtx::get()).unwrap() }

  /// Return the variant of the `ContainerColor` provide in the current build
  /// context and unwrap it as a `Color`.
  pub fn container_color() -> VariantMap<ContainerColor, impl Fn(&ContainerColor) -> Color> {
    Variant::new(BuildCtx::get())
      .unwrap()
      .map(|c: &ContainerColor| c.0)
  }

  /// Return if the widget is still valid.
  pub fn is_valid_widget(&self, id: WidgetId) -> bool { !id.is_dropped(self.tree()) }

  /// Marks the widget as requiring a re-layout, ensuring it will be processed
  /// in the next layout pass.
  ///
  /// Widgets are automatically flagged as dirty upon state changes. This method
  /// allows for explicit marking of the widget as dirty when additional
  /// layout updates are needed outside of standard state changes.
  pub fn dirty(&self, id: WidgetId) {
    let tree = self.tree();
    let scope = id.assert_get(tree).dirty_phase();
    let scope = if scope == DirtyPhase::LayoutSubtree { scope } else { DirtyPhase::Layout };
    tree.dirty_marker().mark(id, scope);
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

  pub(crate) fn tree_ptr(&self) -> NonNull<WidgetTree> { self.tree }

  pub(crate) fn build(&mut self, widget: Widget<'_>) -> WidgetId {
    let size = self.children.len();
    let root = widget.call(self);
    loop {
      if self.children.len() == size {
        break;
      }
      if let Some((p, child)) = self.children.pop() {
        let c = child.call(self);
        p.append(c, self.tree_mut());
      }
    }

    root
  }

  pub(crate) fn build_parent(&mut self, parent: Widget<'_>, children: Vec<Widget<'_>>) -> WidgetId {
    let root = self.build(parent);
    let p = root.single_leaf(self.tree_mut());
    for c in children.into_iter().rev() {
      // Safety: The child will not truly extend its lifetime to 'static; it only
      // exists during the build process, and the parent's lifetime live longer than
      // the build process.
      let c: Widget<'static> = unsafe { std::mem::transmute(c) };
      self.children.push((p, c));
    }
    root
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
    let provider_ctx = ProviderCtx::collect_from(startup, t);
    let ctx = BuildCtx { tree, children: <_>::default(), provider_ctx };

    BuildCtx::set(ctx);
  }

  pub(crate) fn set(ctx: BuildCtx) { unsafe { CTX = Some(LocalSender::new(ctx)) } }

  pub(crate) fn clear() {
    if let Some(ctx) = BuildCtx::try_get() {
      assert!(ctx.children.is_empty());
    }
    unsafe { CTX = None }
  }

  pub(crate) fn empty(tree: NonNull<WidgetTree>) -> Self {
    Self { tree, children: <_>::default(), provider_ctx: <_>::default() }
  }
}

pub(crate) struct BuildCtxInitdGuard;

impl Drop for BuildCtxInitdGuard {
  fn drop(&mut self) { BuildCtx::clear() }
}

impl AsRef<ProviderCtx> for BuildCtx {
  fn as_ref(&self) -> &ProviderCtx { &self.provider_ctx }
}

impl AsMut<ProviderCtx> for BuildCtx {
  fn as_mut(&mut self) -> &mut ProviderCtx { &mut self.provider_ctx }
}
