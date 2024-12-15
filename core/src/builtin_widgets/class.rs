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

use std::{
  cell::{RefCell, UnsafeCell},
  hash::{Hash, Hasher},
};

use ribir_algo::Sc;
use smallvec::{SmallVec, smallvec};
use widget_id::RenderQueryable;

use crate::{
  data_widget::AnonymousAttacher,
  pipe::{DynInfo, DynWidgetsInfo, GenRange},
  prelude::*,
  render_helper::{PureRender, RenderProxy},
  window::WindowId,
};

/// Macro used to define a class to override for a `ClassName`, this is a
/// shorthand if you only want to compose builtin widgets with your host widget.
#[macro_export]
macro_rules! style_class {
($($field: ident: $value: expr),* $(,)?) => {
    (move |widget: Widget| {
      $crate::prelude::FatObj::new(widget) $(.$field($value))* .into_widget()
    }) as $crate::prelude::ClassImpl
  };
}

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

/// A function that transforms a `Widget` into another `Widget` as a class
/// implementation.
///
/// The function accepts a `Widget` as input and returns a
/// `Widget` as output, ensuring that the input widget is retained in the
/// returned widget. Otherwise, switching the class to another class will fail.
pub type ClassImpl = fn(Widget) -> Widget;

/// This widget is used to apply class to its child widget by the `ClassName`.
#[derive(Default)]
pub struct Class {
  pub class: Option<ClassName>,
}

/// This widget overrides the class implementation of a `ClassName`, offering a
/// lighter alternative to `Classes` when you only need to override a single
/// class.
#[simple_declare]
#[derive(Eq)]
pub struct OverrideClass {
  pub name: ClassName,
  pub class_impl: ClassImpl,
}

/// This macro is utilized to define class names; ensure that your name is
/// unique within the application.
#[macro_export]
macro_rules! class_names {
  ($(
    $(#[$outer:meta])?
    $name:ident
  ),* $(,)?) => {
    $(
      $(#[$outer])?
      pub const $name: ClassName = ClassName::new(stringify!($name));
    )*

  };
}

impl ClassName {
  pub const fn new(name: &'static str) -> Self { ClassName(name) }
}

impl Classes {
  #[inline]
  /// Assigns the implementation of `cls` to the store and returns the previous
  /// implementation, if any.
  ///
  /// Note: You must ensure that the widget provided in the `ClassImpl` is
  /// maintained in the returned widget of the `ClassImpl`.
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

impl<'c> ComposeChild<'c> for OverrideClass {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let cls_override = this.try_into_value().unwrap_or_else(|_| {
      panic!("Attempting to use `OverrideClass` as a reader or writer is not allowed.")
    });
    let f = move || {
      BuildCtx::get_mut()
        .current_providers
        .push(Box::new(Queryable(cls_override)));
      let id = child.build();
      BuildCtx::get_mut().current_providers.pop();
      Widget::from_id(id)
    };
    f.into_widget()
  }
}

impl<'c> ComposeChild<'c> for Class {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let f = move || match this.try_into_value() {
      Ok(c) => c.apply_style(child),
      Err(this) => {
        let this2 = this.clone_watcher();
        let cls_child = ClassNode::dummy(BuildCtx::get().tree().dummy_id());
        // Reapply the class when it is updated.
        let cls_child2 = cls_child.clone();
        let child = child.on_build(move |orig_id| {
          let tree = BuildCtx::get_mut().tree_mut();
          let mut orig_child = ClassNode::from_id(orig_id, tree);
          let orig_child2 = orig_child.clone();
          let wnd_id = tree.window().id();
          let u = this2
            .raw_modifies()
            .filter(|s| s.contains(ModifyScope::FRAMEWORK))
            .sample(AppCtx::frame_ticks().clone())
            .subscribe(move |_| cls_child2.update(&orig_child2, &this2.read(), wnd_id))
            .unsubscribe_when_dropped();
          orig_child.attach_subscription(u);
        });

        this
          .read()
          .apply_style(child)
          .on_build(move |child_id| cls_child.init_from_id(child_id))
      }
    };
    f.into_widget()
  }
}

impl Class {
  pub fn apply_style<'a>(&self, w: Widget<'a>) -> Widget<'a> {
    if let Some(cls_impl) = self.class_impl() { cls_impl(w) } else { w }
  }

  fn class_impl(&self) -> Option<ClassImpl> {
    let cls = self.class?;
    let override_cls_id = QueryId::of::<OverrideClass>();
    let classes_id = QueryId::of::<Classes>();

    let (id, handle) = BuildCtx::get().all_providers().find_map(|p| {
      p.query_match(&[override_cls_id, classes_id], &|id, h| {
        if id == &override_cls_id {
          h.downcast_ref::<OverrideClass>()
            .is_some_and(|c| c.name == cls)
        } else {
          h.downcast_ref::<Classes>()
            .is_some_and(|c| c.store.contains_key(&cls))
        }
      })
    })?;

    if id == override_cls_id {
      handle
        .into_ref::<OverrideClass>()
        .map(|cls| cls.class_impl)
    } else {
      let classes = handle.into_ref::<Classes>()?;
      classes.store.get(&cls).cloned()
    }
  }
}

/// This macro is used to define a class implementation by combining multiple
/// other class implementations.
#[macro_export]
macro_rules! multi_class_impl {
  ($($class: expr),*) => {
    move |mut w: Widget| {
      $(w = $class(w);)*
      w
    }
  };
}

#[derive(Clone)]
struct ClassNode(Sc<UnsafeCell<InnerClassNode>>);

struct InnerClassNode {
  render: Box<dyn RenderQueryable>,
  id_info: DynInfo,
}

impl ClassNode {
  fn dummy(id: WidgetId) -> Self {
    let inner = InnerClassNode {
      render: Box::new(PureRender(Void)),
      id_info: Sc::new(RefCell::new(DynWidgetsInfo {
        multi_pos: 0,
        gen_range: GenRange::Single(id),
      })),
    };
    Self(Sc::new(UnsafeCell::new(inner)))
  }

  fn from_id(id: WidgetId, tree: &mut WidgetTree) -> Self {
    let mut orig = None;
    id.wrap_node(tree, |node| {
      let c = ClassNode(Sc::new(UnsafeCell::new(InnerClassNode {
        render: node,
        id_info: Sc::new(RefCell::new(DynWidgetsInfo {
          multi_pos: 0,
          gen_range: GenRange::Single(id),
        })),
      })));
      orig = Some(c.clone());
      Box::new(c)
    });

    orig.unwrap()
  }

  fn init_from_id(&self, id: WidgetId) {
    let inner = self.inner();
    inner.id_info.borrow_mut().gen_range = GenRange::Single(id);
    id.wrap_node(BuildCtx::get_mut().tree_mut(), |node| {
      inner.render = node;

      Box::new(self.clone())
    });
  }

  fn update(&self, orig: &ClassNode, class: &Class, wnd_id: WindowId) {
    let wnd =
      AppCtx::get_window(wnd_id).expect("This handle is not valid because the window is closed");
    let child_id = self.id();
    let orig_id = orig.id();
    let InnerClassNode { render: child, id_info } = self.inner();
    let _guard = BuildCtx::init_for(child_id, wnd.tree);
    let n_orig = BuildCtx::get_mut().alloc(Box::new(orig.clone()));
    let tree = BuildCtx::get_mut().tree_mut();
    let cls_holder = child_id.place_holder(tree);

    // Extract the child from this node, retaining only the external information
    // linked from the parent to create a clean context for applying the class.
    let child_node = self.take_inner();
    let mut new_id = class.apply_style(Widget::from_id(n_orig)).build();

    // Place the inner child node within the old ID for disposal, then utilize the
    // class node to wrap the new child in the new ID.
    // This action should be taken before modifying the `orig_id`, as the `orig_id`
    // may be the same as the `child_id`.
    let class_node = std::mem::replace(child_id.get_node_mut(tree).unwrap(), child_node);

    // Retain the original widget ID.
    let [new, old] = tree.get_many_mut(&[n_orig, orig_id]);
    std::mem::swap(new, old);
    if new_id == n_orig {
      // If applying the class does not generate additional widgets, the original
      // widget ID will include all new elements after the swap.
      new_id = orig_id;
    } else {
      n_orig.insert_after(orig_id, tree);
      tree.remove_subtree(n_orig);
    }

    if child_id != new_id {
      // update the DynamicWidgetId out of the class node when id changed.
      let mut v = SmallVec::new();
      class_node.query_all_write(&QueryId::of::<TrackId>(), &mut v);
      v.into_iter()
        .filter_map(QueryHandle::into_ref)
        .for_each(|handle: QueryRef<'_, TrackId>| {
          handle.set(Some(new_id));
        });
    }

    new_id.wrap_node(tree, |node| {
      *child = node;
      class_node
    });

    if new_id != child_id {
      // If a pipe widget generates a widget with a class, we place the pipe node
      // outside of the class node. However, since its widget ID is altered, we must
      // notify the pipe node accordingly.
      let old_rg = child_id..=orig_id;
      let new_rg = new_id..=orig_id;
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
      if w != child_id {
        w.on_mounted_subtree(tree);
        stack.extend(w.children(tree).rev());
      }
    }

    new_id
      .query_all_iter::<TrackId>(tree)
      .for_each(|wid| {
        wid.set(Some(new_id));
      });

    id_info.borrow_mut().gen_range = GenRange::Single(new_id);
    let marker = tree.dirty_marker();
    marker.mark(new_id);
    if new_id != orig_id && new_id.ancestor_of(orig_id, tree) {
      marker.mark(orig_id);
    }
  }

  #[allow(clippy::mut_from_ref)]
  fn inner(&self) -> &mut InnerClassNode { unsafe { &mut *self.0.get() } }

  fn take_inner(&self) -> Box<dyn RenderQueryable> {
    std::mem::replace(&mut self.inner().render, Box::new(PureRender(Void)))
  }

  fn attach_subscription(&mut self, guard: impl Any) {
    let inner = &mut self.inner().render;
    let child = unsafe { Box::from_raw(inner.as_mut()) };
    let child = Box::new(AnonymousAttacher::new(child, Box::new(guard)));
    let tmp = std::mem::replace(inner, child);
    std::mem::forget(tmp);
  }

  fn id(&self) -> WidgetId { self.inner().id_info.borrow().host_id() }
}

impl RenderProxy for ClassNode {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.inner().render.as_ref() }
}

impl Query for ClassNode {
  fn query_all<'q>(
    &'q self, query_id: &QueryId, out: &mut smallvec::SmallVec<[QueryHandle<'q>; 1]>,
  ) {
    let inner = self.inner();
    inner.render.query_all(query_id, out);
    if query_id == &QueryId::of::<DynInfo>() {
      out.push(QueryHandle::new(&inner.id_info));
    }
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.inner().render.query_all_write(query_id, out)
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    let inner = self.inner();
    if query_id == &QueryId::of::<DynInfo>() {
      Some(QueryHandle::new(&inner.id_info))
    } else {
      inner.render.query(query_id)
    }
  }

  fn query_match(
    &self, ids: &[QueryId], filter: &dyn Fn(&QueryId, &QueryHandle) -> bool,
  ) -> Option<(QueryId, QueryHandle)> {
    let inner = self.inner();
    inner.render.query_match(ids, filter).or_else(|| {
      let dyn_info_id = QueryId::of::<DynInfo>();
      (ids.contains(&dyn_info_id))
        .then(|| {
          let h = QueryHandle::new(&inner.id_info);
          filter(&dyn_info_id, &h).then_some((dyn_info_id, h))
        })
        .flatten()
    })
  }

  fn query_write(&self, type_id: &QueryId) -> Option<QueryHandle> {
    self.inner().render.query_write(type_id)
  }
}

impl PartialEq for OverrideClass {
  fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

impl Hash for OverrideClass {
  fn hash<H: Hasher>(&self, state: &mut H) { self.name.hash(state); }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    reset_test_env,
    test_helper::{MockBox, MockMulti, TestWindow, split_value},
  };
  class_names!(MARGIN, BOX_200, CLAMP_50, EMPTY);

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
    classes.insert(CLAMP_50, style_class! {
      clamp: BoxClamp::fixed_size(Size::new(50., 50.))
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
    wnd.assert_root_size(Size::splat(120.));

    *w_cls.write() = BOX_200;
    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(200.));

    *w_cls.write() = MARGIN;
    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(120.));
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
    wnd.assert_root_size(Size::splat(100.));

    *w_cls.write() = BOX_200;
    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(200.));
  }

  #[test]
  fn fix_style_class_switch() {
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

    *w_cls.write() = CLAMP_50;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(50., 50.));
  }

  #[test]
  fn override_class() {
    reset_test_env!();

    let mut wnd = TestWindow::new(fn_widget! {
      initd_classes().with_child(fn_widget! {
        @OverrideClass {
          name: MARGIN,
          class_impl: style_class! {
            clamp: BoxClamp::fixed_size(Size::new(66., 66.))
          } as ClassImpl,
          @Container {
            size: Size::new(100., 100.),
            class: MARGIN,
          }
        }
      })
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(66., 66.));
  }

  #[test]
  fn fix_pipe_class_on_pipe_widget() {
    reset_test_env!();

    let (w_trigger, w) = split_value(true);
    let (cls, w_cls) = split_value(EMPTY);

    let mut wnd = TestWindow::new(fn_widget! {
      let w_trigger = w_trigger.clone_watcher();
      let cls = cls.clone_watcher();
      initd_classes().with_child(fn_widget! {
        let w = pipe!(*$w_trigger).map(|_|{
          @Container {size: Size::new(100., 100.) }
        });
        let w = FatObj::new(w);
        @ $w { class: pipe!(*$cls) }
      })
    });

    wnd.draw_frame();
    *w.write() = false;
    wnd.draw_frame();
    *w_cls.write() = MARGIN;
    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(120.));
  }
}
