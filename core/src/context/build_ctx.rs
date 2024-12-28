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
  /// Providers are available for current building
  pub(crate) current_providers: SmallVec<[Box<dyn Query>; 1]>,
  pub(crate) tree: NonNull<WidgetTree>,
  // Todo: Since `Theme`, `Palette`, `TypographyTheme` and `TextStyle` are frequently queried
  // during the building process, layout and paint. we should cache the closest one.
  /// The stack is utilized to temporarily store the children's relationships
  /// during the build process. The widget's lifetime remains valid throughout
  /// this process; hence, we use 'static to avoid introducing a lifetime for
  /// the BuildCtx.
  pub(crate) children: Vec<(WidgetId, Widget<'static>)>,
}

impl BuildCtx {
  /// Return the window of this context is created from.
  pub fn window(&self) -> Sc<Window> { self.tree().window() }

  pub fn text_style(&self) -> QueryRef<TextStyle> { Provider::of::<TextStyle>(self).unwrap() }

  /// This method returns the color of the current build process, with the
  /// primary color of the palette serving as the default.
  ///
  /// This color is used for interactions between the user and the widget theme.
  /// For instance, the `Button` widget does not have a `color` property, but
  /// its class utilizes the color returned by this method to style the button.
  /// Therefore, users can modify this color to change the button's color
  /// without having to override its class.
  pub fn variant_color(&self) -> Color {
    // todo: We have not yet enabled the ability to change the variant color. This
    // method is provided for compatibility purposes now.
    Palette::of(self).primary()
  }

  /// The container color of the variant color.
  pub fn variant_container_color(&self) -> Color { Palette::of(self).secondary_container() }

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

  pub(crate) fn build_with_provider(
    &mut self, widget: Widget<'_>, provider: Box<dyn Query>,
  ) -> WidgetId {
    // We push the provider into the build context to ensure that the widget build
    // logic can access this provider.
    self.current_providers.push(provider);
    let id = self.build(widget.into_widget());
    let provider = self.current_providers.pop().unwrap();
    // Attach the provider to the widget so its descendants can access it.
    id.attach_data(provider, self.tree_mut());
    id
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
    let mut providers: SmallVec<[WidgetId; 1]> = startup
      .ancestors(t)
      .filter(|id| id.queryable(t))
      .collect();
    providers.reverse();
    let ctx =
      BuildCtx { tree, providers, current_providers: <_>::default(), children: <_>::default() };
    BuildCtx::set(ctx);
  }

  pub(crate) fn set(ctx: BuildCtx) { unsafe { CTX = Some(LocalSender::new(ctx)) } }

  pub(crate) fn clear() {
    if let Some(ctx) = BuildCtx::try_get() {
      assert!(ctx.children.is_empty());
    }
    unsafe { CTX = None }
  }
}

pub(crate) struct BuildCtxInitdGuard;

impl Drop for BuildCtxInitdGuard {
  fn drop(&mut self) { BuildCtx::clear() }
}
