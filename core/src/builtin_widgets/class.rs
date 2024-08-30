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
//! theme.classes.insert(
//!   RED_BORDER,
//!   style_class! {
//!     border: Border::all(BorderSide::new(2., Color::RED.into()))
//!   },
//! );
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
      .with_child(fn_widget! {
        pipe!($this;).map(move |_| child.gen_widget())
      })
      .into_widget()
  }
}

impl<'c> ComposeChild<'c> for Class {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let f = move |ctx: &mut BuildCtx| match this.try_into_value() {
      Ok(c) => c.apply_style(child, ctx),
      Err(this) => {
        let (child, orig_id) = child.consume_root(ctx);
        let mut orig_child = OrigChild::from_id(orig_id, ctx);
        let cls_child = ClassChild::new(orig_id);

        let handle = ctx.handle();
        let reader = this.clone_reader();
        // Reapply the class when it is updated.
        let cls_child2 = cls_child.clone();
        let orig_child2 = orig_child.clone();

        let u = this
          .raw_modifies()
          .filter(|s| s.contains(ModifyScope::FRAMEWORK))
          .sample(AppCtx::frame_ticks().clone())
          .subscribe(move |_| {
            handle.with_ctx(|ctx| cls_child2.update(&orig_child2, &reader.read(), ctx));
          });
        orig_child.attach_subscription(u);

        let (child, child_id) = this
          .read()
          .apply_style(child, ctx)
          .consume_root(ctx);

        cls_child.set_child_id(child_id, ctx);

        child
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

    // Revert back to class node only,  maybe there is information attached on it,
    // that we need keep.
    let class_node =
      std::mem::replace(child_id.get_node_mut(ctx.tree_mut()).unwrap(), Box::new(self.clone()));

    // Revert to the original child.
    *orig_id.get_node_mut(ctx.tree_mut()).unwrap() = Box::new(orig.clone());
    let new_id = class
      .apply_style(Widget::from_id(*orig_id), ctx)
      .build(ctx);

    let tree = ctx.tree_mut();
    new_id.wrap_node(tree, |node| {
      *child = node;
      class_node
    });

    if new_id != *child_id {
      // If we modify the tree structure, we must notify the pipe accordingly.
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
    }

    if orig_id != child_id {
      child_id.insert_after(new_id, tree);
      child_id.dispose_subtree(tree);
    }

    if new_id != *orig_id {
      let mut w = new_id;
      loop {
        w.on_widget_mounted(tree);
        if w == *orig_id {
          break;
        }
        w = w.single_child(tree).unwrap();
      }
    }
    *child_id = new_id;

    tree.mark_dirty(new_id);
  }

  #[allow(clippy::mut_from_ref)]
  fn inner(&self) -> &mut InnerClassChild { unsafe { &mut *self.0.get() } }
}

impl OrigChild {
  fn from_id(id: WidgetId, ctx: &mut BuildCtx) -> Self {
    let mut cls = None;
    id.wrap_node(ctx.tree_mut(), |node| {
      let c = OrigChild(Sc::new(UnsafeCell::new(node)));
      cls = Some(c.clone());
      Box::new(c)
    });

    cls.unwrap()
  }

  fn attach_subscription(&mut self, guard: impl Any) {
    let inner = self.node();
    let child = unsafe { Box::from_raw(inner.as_mut()) };
    let child = Box::new(AnonymousAttacher::new(child, Box::new(guard)));
    let tmp = std::mem::replace(inner, child);
    std::mem::forget(tmp);
  }

  #[allow(clippy::mut_from_ref)]
  fn node(&self) -> &mut Box<dyn RenderQueryable> { unsafe { &mut (*self.0.get()) } }
}

impl RenderProxy for OrigChild {
  type Target<'r> = &'r dyn RenderQueryable
      where
        Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { unsafe { &**self.0.get() } }
}

impl RenderProxy for ClassChild {
  type Target<'r> = &'r dyn RenderQueryable
      where
        Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.inner().child.as_ref() }
}

impl Query for OrigChild {
  fn query_all<'q>(&'q self, type_id: TypeId, out: &mut smallvec::SmallVec<[QueryHandle<'q>; 1]>) {
    self.proxy().query_all(type_id, out)
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> { self.proxy().query(type_id) }

  fn query_write(&self, type_id: TypeId) -> Option<QueryHandle> {
    self.proxy().query_write(type_id)
  }
}

impl Query for ClassChild {
  fn query_all<'q>(&'q self, type_id: TypeId, out: &mut smallvec::SmallVec<[QueryHandle<'q>; 1]>) {
    self.proxy().query_all(type_id, out)
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> { self.proxy().query(type_id) }

  fn query_write(&self, type_id: TypeId) -> Option<QueryHandle> {
    self.proxy().query_write(type_id)
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::assert_layout_result_by_path;

  use super::*;
  use crate::{
    reset_test_env,
    test_helper::{split_value, MockBox, TestWindow},
  };

  #[test]
  fn switch_class() {
    reset_test_env!();

    class_names!(MARGIN, SCALE_2X);

    let mut theme = Theme::default();

    theme
      .classes
      .insert(MARGIN, |w| fn_widget! { @ $w{ margin: EdgeInsets::all(10.) } }.into_widget());
    theme.classes.insert(SCALE_2X, |w| {
      fn_widget! {
        @MockBox {
          size: Size::new(200., 200.),
          @ { w }
        }
      }
      .into_widget()
    });

    unsafe { AppCtx::set_app_theme(theme) };

    let (cls, w_cls) = split_value(MARGIN);

    let mut wnd = TestWindow::new(fn_widget! {
      @Container {
        size: Size::new(100., 100.),
        class: pipe!(*$cls),
      }
    });

    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(120., 120.),});

    *w_cls.write() = SCALE_2X;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(200., 200.),});
  }
}
