//! `Class` is a widget used to specify a class for its child widget. It draws
//! inspiration from the HTML class attribute, enabling the sharing of
//! consistent styles across multiple elements with different functionalities.
//!
//! In Ribir, you can define a class name using `class_name!`, which the `Class`
//! widget can utilize to locate a function in `Theme` or `Classes` to transform
//! its child into another widget. This means that `Class` not only facilitates
//! style sharing but also allows for the addition of action behavior or
//! composition with multiple widgets.
//!
//! # Example
//!
//! ```no_run
//! use ribir::prelude::*;
//!
//! class_names!(RED_BORDER);
//!
//! let mut theme = Theme::default();
//! // Define how `RED_BORDER` transforms a widget.
//! theme.classes.insert(RED_BORDER, style_class! {
//!   border: Border::all(BorderSide::new(2., Color::RED.into()))
//! });
//!
//! let w = fn_widget! {
//!   @Container {
//!     size: Size::new(100., 100.),
//!     class: RED_BORDER,
//!   }
//! };
//!
//! App::run(w).with_app_theme(theme);
//! ```

use std::cell::UnsafeCell;

use ribir_algo::Sc;
use smallvec::{SmallVec, smallvec};
use widget_id::RenderQueryable;

use crate::{
  data_widget::AnonymousAttacher,
  pipe::DynInfo,
  prelude::*,
  render_helper::{PureRender, RenderProxy},
};

/// A collection comprises the implementations of the `ClassName`, offering the
/// implementation of `Class` within its descendants.

#[derive(Default)]
pub struct Classes {
  pub(crate) store: ahash::HashMap<ClassName, ClassImpl>,
}

/// This type is utilized to define a constant variable as the name of a
/// `Class`. It can also override its implementation across the `Theme` and
/// `Classes`.
///
/// # Example
///
/// Use `class_names!` to define your class names.
///
/// ```
/// use ribir::prelude::*;
///
/// class_names!(A, B, C);
/// ```
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ClassName(&'static str);

pub type ClassImpl = fn(Widget) -> Widget;

/// This widget is used to apply class to its child widget by the `ClassName`.
#[derive(Default)]
pub struct Class {
  pub class: Option<ClassName>,
}

/// This macro is utilized to define class names; ensure that your name is
/// unique within the application.
#[macro_export]
macro_rules! class_names {
  ($(
    $(#[$outer:meta])?
    $name:ident
  ),*) => {
    $(
      $(#[$outer])?
      pub const $name: ClassName = ClassName::new(stringify!($name:snake));
    )*

  };
}

impl ClassName {
  pub const fn new(name: &'static str) -> Self { ClassName(name) }
}

impl Classes {
  #[inline]
  pub fn insert(&mut self, cls: ClassName, f: ClassImpl) -> Option<ClassImpl> {
    self.store.insert(cls, f)
  }
}

impl Declare for Class {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl ComposeChild<'static> for Classes {
  type Child = GenWidget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    Provider::new(Box::new(this.clone_writer()))
      .with_child(fn_widget! { pipe!($this;).map(move |_| child.gen_widget())})
      .into_widget()
  }
}

impl<'c> ComposeChild<'c> for Class {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let f = move |ctx: &mut BuildCtx| match this.try_into_value() {
      Ok(c) => c.apply_style(child, ctx),
      Err(this) => {
        let this2 = this.clone_watcher();
        let cls_child = ClassChild::new(ctx.tree().dummy_id());
        // Reapply the class when it is updated.
        let cls_child2 = cls_child.clone();
        let child = child.on_build(move |orig_id, ctx| {
          let mut orig_child = OrigChild::from_id(orig_id, ctx);
          cls_child2.inner().orig_id = orig_id;
          let orig_child2 = orig_child.clone();
          let handle = ctx.handle();

          let u = this2
            .raw_modifies()
            .filter(|s| s.contains(ModifyScope::FRAMEWORK))
            .sample(AppCtx::frame_ticks().clone())
            .subscribe(move |_| {
              handle.with_ctx(|ctx| cls_child2.update(&orig_child2, &this2.read(), ctx));
            })
            .unsubscribe_when_dropped();
          orig_child.attach_subscription(u);
        });

        this
          .read()
          .apply_style(child, ctx)
          .on_build(move |child_id, ctx| cls_child.set_child_id(child_id, ctx))
      }
    };
    f.into_widget()
  }
}

impl Class {
  pub fn apply_style<'a>(&self, w: Widget<'a>, ctx: &BuildCtx) -> Widget<'a> {
    let class = self.class.and_then(|cls| {
      ctx
        .all_providers::<Classes>()
        .find_map(|c| QueryRef::filter_map(c, |c| c.store.get(&cls)).ok())
    });
    if let Some(c) = class { c(w) } else { w }
  }
}

/// This macro is used to define a class implementation by combining multiple
/// other class implementations.
#[macro_export]
macro_rules! multi_class {
  ($($class: expr),*) => {
    move |mut w: Widget| {
      $(w = $class(w);)*
      w
    }
  };
}

#[derive(Clone)]
struct OrigChild(Sc<UnsafeCell<Box<dyn RenderQueryable>>>);

#[derive(Clone)]
struct ClassChild(Sc<UnsafeCell<InnerClassChild>>);

struct InnerClassChild {
  child: Box<dyn RenderQueryable>,
  child_id: WidgetId,
  orig_id: WidgetId,
}

impl ClassChild {
  fn new(id: WidgetId) -> Self {
    let inner = InnerClassChild { child: Box::new(PureRender(Void)), child_id: id, orig_id: id };
    Self(Sc::new(UnsafeCell::new(inner)))
  }

  fn set_child_id(&self, id: WidgetId, ctx: &mut BuildCtx) {
    let inner = self.inner();
    inner.child_id = id;
    id.wrap_node(ctx.tree_mut(), |node| {
      inner.child = node;
      Box::new(self.clone())
    });
  }

  fn update(&self, orig: &OrigChild, class: &Class, ctx: &mut BuildCtx) {
    let InnerClassChild { child, child_id, orig_id } = self.inner();
    eprintln!("old tree\n {}", ctx.tree().display_tree(ctx.tree().root()));
    let cls_holder = child_id.place_holder(ctx.tree());

    // Extract the child from this node, retaining only the external information
    // linked from the parent to create a clean context for applying the class.
    let child_node = self.take_inner();
    let n_orig = ctx.alloc(Box::new(orig.clone()));
    let new_id = class
      .apply_style(Widget::from_id(n_orig), ctx)
      .build(ctx);
    let tree = ctx.tree_mut();
    // Place the inner child node within the old ID for disposal, then utilize the
    // class node to wrap the new child in the new ID.
    // This action should be taken before modifying the `orig_id`, as the `orig_id`
    // may be the same as the `child_id`.
    let class_node = std::mem::replace(child_id.get_node_mut(tree).unwrap(), child_node);

    // Retain the original widget ID.
    let [new, old] = tree.get_many_mut(&[n_orig, *orig_id]);
    std::mem::swap(new, old);
    n_orig.insert_after(*orig_id, tree);
    n_orig.dispose_subtree(tree);

    new_id.wrap_node(tree, |node| {
      *child = node;
      class_node
    });

    if new_id != *child_id {
      // If a pipe widget generates a widget with a class, we place the pipe node
      // outside of the class node. However, since its widget ID is altered, we must
      // notify the pipe node accordingly.
      let old_rg = *child_id..=*orig_id;
      let new_rg = new_id..=*orig_id;
      new_id
        .query_all_iter::<DynInfo>(tree)
        .rev()
        .for_each(|info| {
          info
            .borrow_mut()
            .single_range_replace(&old_rg, &new_rg)
        });
      cls_holder.replace(new_id, tree);
    }

    if orig_id != child_id {
      child_id.dispose_subtree(tree);
    }

    let mut stack: SmallVec<[WidgetId; 1]> = smallvec![new_id];
    while let Some(w) = stack.pop() {
      // Skip the original child subtree as it does not consist of new widgets.
      if w != *orig_id {
        w.on_mounted_subtree(tree);
        stack.extend(w.children(tree).rev());
      }
    }

    *child_id = new_id;
    tree.mark_dirty(new_id);
    tree.mark_dirty(*orig_id);
    eprintln!("new tree\n {}", ctx.tree().display_tree(ctx.tree().root()));
  }

  #[allow(clippy::mut_from_ref)]
  fn inner(&self) -> &mut InnerClassChild { unsafe { &mut *self.0.get() } }

  fn take_inner(&self) -> Box<dyn RenderQueryable> {
    std::mem::replace(&mut self.inner().child, Box::new(PureRender(Void)))
  }
}

impl OrigChild {
  fn from_id(id: WidgetId, ctx: &mut BuildCtx) -> Self {
    let mut orig = None;
    id.wrap_node(ctx.tree_mut(), |node| {
      let c = OrigChild(Sc::new(UnsafeCell::new(node)));
      orig = Some(c.clone());
      Box::new(c)
    });

    orig.unwrap()
  }

  fn attach_subscription(&mut self, guard: impl Any) {
    let inner = self.node_mut();
    let child = unsafe { Box::from_raw(inner.as_mut()) };
    let child = Box::new(AnonymousAttacher::new(child, Box::new(guard)));
    let tmp = std::mem::replace(inner, child);
    std::mem::forget(tmp);
  }

  fn node(&self) -> &dyn RenderQueryable { unsafe { &*(*self.0.get()) } }

  #[allow(clippy::mut_from_ref)]
  fn node_mut(&self) -> &mut Box<dyn RenderQueryable> { unsafe { &mut (*self.0.get()) } }
}

impl RenderProxy for OrigChild {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.node() }
}

impl RenderProxy for ClassChild {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.inner().child.as_ref() }
}

impl Query for OrigChild {
  fn query_all<'q>(&'q self, type_id: TypeId, out: &mut smallvec::SmallVec<[QueryHandle<'q>; 1]>) {
    self.node().query_all(type_id, out)
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> { self.node().query(type_id) }

  fn query_write(&self, type_id: TypeId) -> Option<QueryHandle> {
    self.node_mut().query_write(type_id)
  }
}

impl Query for ClassChild {
  fn query_all<'q>(&'q self, type_id: TypeId, out: &mut smallvec::SmallVec<[QueryHandle<'q>; 1]>) {
    self.inner().child.query_all(type_id, out)
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> { self.inner().child.query(type_id) }

  fn query_write(&self, type_id: TypeId) -> Option<QueryHandle> {
    self.inner().child.query_write(type_id)
  }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::{
    reset_test_env,
    test_helper::{MockBox, MockMulti, TestWindow, split_value},
  };
  class_names!(MARGIN, BOX_200, EMPTY);

  fn initd_classes() -> Classes {
    let mut classes = Classes::default();
    classes.insert(MARGIN, style_class!(margin: EdgeInsets::all(10.)));
    classes.insert(BOX_200, |w| {
      fn_widget! {
        @MockBox {
          size: Size::new(200., 200.),
          @ { w }
        }
      }
      .into_widget()
    });
    classes
  }

  #[test]
  fn switch_class() {
    reset_test_env!();

    let (cls, w_cls) = split_value(MARGIN);
    let mut wnd = TestWindow::new(fn_widget! {
      let cls = cls.clone_watcher();
      initd_classes().with_child(fn_widget! {
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      })
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(120., 120.));

    *w_cls.write() = BOX_200;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(200., 200.));
  }

  #[test]
  #[should_panic(expected = "on_disposed called")]
  fn on_disposed_of_class_nodes() {
    reset_test_env!();

    class_names!(ON_DISPOSED);

    let (cls, w_cls) = split_value(ON_DISPOSED);

    let mut wnd = TestWindow::new(fn_widget! {
      let cls = cls.clone_watcher();
      let mut classes = initd_classes();
      classes.insert(ON_DISPOSED, |w| {
        fn_widget! {
          @MockBox {
            size: Size::zero(),
            on_disposed: move |_| panic!("on_disposed called"),
            @ { w }
          }
        }
        .into_widget()
      });
      classes.with_child(fn_widget! {
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      })
    });

    wnd.draw_frame();
    *w_cls.write() = MARGIN;
    wnd.draw_frame();
  }

  #[test]
  fn fix_crash_for_class_impl_may_have_multi_child() {
    reset_test_env!();

    class_names!(MULTI);

    let (cls, w_cls) = split_value(MARGIN);

    let mut wnd = TestWindow::new(fn_widget! {
      let cls = cls.clone_watcher();
      let mut classes = initd_classes();
      classes.insert(MULTI, |w| {
        fn_widget! {
          @MockMulti {
            @MockBox { size: Size::new(100., 100.) }
            @MockBox { size: Size::new(100., 200.) }
            @ { w }
          }
        }
        .into_widget()
      });
      classes.with_child(fn_widget! {
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      })
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(120., 120.));

    *w_cls.write() = MULTI;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(300., 200.));
  }

  #[test]
  #[should_panic(expected = "0")]
  fn fix_provider_in_pipe_class() {
    reset_test_env!();

    class_names!(PROVIDER_CLS);

    let mut wnd = TestWindow::new(fn_widget! {
      let trigger = Stateful::new(true);
      let mut classes = Classes::default();
      classes.insert(PROVIDER_CLS, |w| {
        Provider::new(Box::new(Queryable(0)))
          .with_child(fn_widget! { w })
          .into_widget()
      });
      classes.with_child(fn_widget! {
        @Container {
          size: Size::new(100., 100.),
          class: pipe!($trigger; PROVIDER_CLS),
          on_performed_layout: |e| {
            panic!("{}", *e.query::<i32>().unwrap());
          }
        }
      })
    });
    wnd.draw_frame();
  }

  #[test]
  fn fix_not_mounted_class_node() {
    reset_test_env!();

    let (cls, w_cls) = split_value(EMPTY);
    let mut wnd = TestWindow::new(fn_widget! {
      let cls = cls.clone_watcher();
      initd_classes().with_child(fn_widget! {
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      })
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(100., 100.));

    *w_cls.write() = BOX_200;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(200., 200.));
  }
}
