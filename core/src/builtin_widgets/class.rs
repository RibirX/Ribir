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

use std::{convert::Infallible, hash::Hash};

use data_widget::AnonymousAttacher;
use ops::box_it::CloneableBoxOp;
use pipe::PipeNode;
use smallvec::{SmallVec, smallvec};

use crate::{pipe::GenRange, prelude::*, window::WindowId};

/// A collection of class implementations that are part of the `Theme`.
#[derive(Default, Clone)]
pub struct Classes {
  pub(crate) store: ahash::HashMap<ClassName, ClassImpl>,
}

/// The macro is used to create a class implementation by accepting declarations
/// of the built-in widget fields.
#[macro_export]
macro_rules! style_class {
($($field: ident: $value: expr),* $(,)?) => {
    (move |widget: $crate::prelude::Widget| {
      $crate::prelude::FatObj::new(widget) $(.$field($value))* .into_widget()
    }) as $crate::prelude::ClassImpl
  };
}

/// The macro is used to create a class implementation with a specified name
/// while accepting declarations of the built-in widget fields.
#[macro_export]
macro_rules! named_style_class {
  ($name: ident => { $($field: ident: $value: expr),* $(,)? }) => {
    fn $name(widget: $crate::prelude::Widget) -> $crate::prelude::Widget {
      $crate::prelude::FatObj::new(widget) $(.$field($value))* .into_widget()
    }
  };
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

/// A empty class implementation that returns the input widget as is.
pub fn empty_cls(w: Widget) -> Widget { w }

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
/// The function accepts a `Widget` as input and returns a `Widget` as output,
/// ensuring that the input widget is retained in the returned widget.
/// Otherwise, switching the class to another class will fail.
// Note: The provider of `Class` can be tricky, so changing the definition of
// ClassImpl without being careful may result in compatibility issues with the
// previous version's binary. See `[`ClassName::type_info`]`.
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
  ),* $(,)?) => {
    $(
      $(#[$outer])?
      pub const $name: ClassName = ClassName::new(stringify!($name));
    )*

  };
}

impl ClassName {
  pub const fn new(name: &'static str) -> Self { ClassName(name) }

  fn type_info(&self) -> TypeInfo {
    const LAYOUT: std::alloc::Layout = std::alloc::Layout::new::<ClassImpl>();
    // Tricky: We disregard the package version since the type remains stable.
    // Instead, we include the class name in the type information, allowing each
    // unique class name to serve as a distinct provider.
    TypeInfo { name: std::any::type_name::<ClassName>(), pkg_version: self.0, layout: &LAYOUT }
  }

  fn from_info(info: &TypeInfo) -> Option<Self> {
    if info.name == std::any::type_name::<ClassName>() {
      Some(Self(info.pkg_version))
    } else {
      None
    }
  }
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

  pub(crate) fn reader_into_provider<R: StateReader<Value = Classes> + Query>(this: R) -> Provider {
    Provider::Setup(Box::new(ClassesReaderSetup(this)))
  }

  fn remove_intersects_class(&self, map: &mut ProviderCtx) -> Vec<(TypeInfo, Box<dyn Query>)> {
    map.remove_key_value_if(|info| {
      ClassName::from_info(info).is_some_and(|name| self.store.contains_key(&name))
    })
  }
}

struct ClassesReaderSetup<T>(T);

struct ClassesRestore {
  overrides: Vec<(TypeInfo, Box<dyn Query>)>,
  classes: Box<dyn ProviderRestore>,
}

impl ProviderSetup for Classes {
  fn setup(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderRestore> {
    let overrides = self.remove_intersects_class(map);
    let classes = Box::new(Setup::new(*self)).setup(map);
    Box::new(ClassesRestore { overrides, classes })
  }

  fn unzip(
    self: Box<Self>,
  ) -> (Box<dyn ProviderSetup>, DirtyPhase, CloneableBoxOp<'static, ModifyScope, Infallible>) {
    unreachable!();
  }
}

impl<R: StateReader<Value = Classes> + Query> ProviderSetup for ClassesReaderSetup<R> {
  fn setup(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderRestore> {
    let classes = self.0;
    let overrides = classes.read().remove_intersects_class(map);
    let classes = Box::new(Setup::from_state(classes)).setup(map);
    Box::new(ClassesRestore { overrides, classes })
  }

  fn unzip(
    self: Box<Self>,
  ) -> (Box<dyn ProviderSetup>, DirtyPhase, CloneableBoxOp<'static, ModifyScope, Infallible>) {
    unreachable!();
  }
}

impl ProviderRestore for ClassesRestore {
  fn restore(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderSetup> {
    let setup = self.classes.restore(map);
    for (info, provider) in self.overrides {
      let _old = map.set_raw_provider(info, provider);
      debug_assert!(_old.is_none());
    }
    setup
  }
}

impl Declare for Class {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for Class {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let f = move || match this.try_into_value() {
      Ok(c) => c.apply_style(child),
      Err(this) => {
        let this2 = this.clone_watcher();
        let cls_child = ClassNode::empty_node();
        // Reapply the class when it is updated.
        let cls_child2 = cls_child.clone();
        let child = child.on_build(move |orig_id| {
          let orig_child = ClassNode::empty_node();
          orig_child.init_for_single(orig_id);
          let orig_child2 = orig_child.clone();
          let wnd_id = BuildCtx::get().window().id();
          let u = this2
            .raw_modifies()
            .filter(|s| s.contains(ModifyScope::FRAMEWORK))
            .sample(AppCtx::frame_ticks().clone())
            .subscribe(move |_| class_update(&cls_child2, &orig_child2, &this2.read(), wnd_id))
            .unsubscribe_when_dropped();
          orig_child.combine_subscription_guard(u);
        });

        this
          .read()
          .apply_style(child)
          .on_build(move |child_id| cls_child.init_for_single(child_id))
      }
    };
    FnWidget::new(f).into_widget()
  }
}

impl Class {
  /// Creating a provider for a class, which can either provide the
  /// implementation of a class or be used to override the implementation of a
  /// class.
  ///
  /// ## Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// class_names!(RED_BOX);
  /// let w = providers! {
  ///   providers: [
  ///     Class::provider(RED_BOX, style_class!{
  ///       background: Color::RED,
  ///       clamp: BoxClamp::fixed_size(Size::splat(48.))
  ///     }),
  ///   ],
  ///   @Void { class: RED_BOX }
  /// };
  /// ```
  pub fn provider(name: ClassName, cls_impl: ClassImpl) -> Provider {
    let setup = Setup::custom(name.type_info(), Box::new(Queryable(cls_impl)));
    Provider::Setup(Box::new(setup))
  }

  fn apply_style<'a>(&self, w: Widget<'a>) -> Widget<'a> {
    if let Some(cls_impl) = self.class_impl() { cls_impl(w) } else { w }
  }

  fn class_impl(&self) -> Option<ClassImpl> {
    let cls = self.class?;
    let ctx = BuildCtx::get();
    let override_cls = ctx
      .as_ref()
      .get_raw_provider(&cls.type_info())
      .and_then(|q| q.query(&QueryId::of::<ClassImpl>()))
      .and_then(QueryHandle::into_ref::<ClassImpl>)
      .map(|i| *i);

    override_cls.or_else(|| {
      Provider::of::<Classes>(ctx)?
        .store
        .get(&cls)
        .copied()
    })
  }
}

type ClassNode = PipeNode;

impl ClassNode {
  fn combine_subscription_guard(&self, guard: impl Any) {
    let inner = self.host_render();
    let child = unsafe { Box::from_raw(inner.as_mut()) };
    let child = Box::new(AnonymousAttacher::new(child, Box::new(guard)));
    let tmp = std::mem::replace(inner, child);
    std::mem::forget(tmp);
  }
}

fn class_update(node: &ClassNode, orig: &ClassNode, class: &Class, wnd_id: WindowId) {
  let wnd =
    AppCtx::get_window(wnd_id).expect("This handle is not valid because the window is closed");

  let child_id = node.dyn_info().host_id();
  let orig_id = orig.dyn_info().host_id();
  if child_id.is_dropped(wnd.tree()) {
    return;
  }

  let child_holder = child_id.place_holder(wnd.tree_mut());

  // Extract the child from this node, retaining only the external information
  // linked from the parent to create a clean context for applying the class.
  let old_child_node = node.take_data();
  let _guard = BuildCtx::init_for(child_id, wnd.tree);
  let ctx = BuildCtx::get_mut();

  // Place the inner child node within the old ID for disposal, then utilize the
  // class node to wrap the new child in the new ID.
  let class_node =
    std::mem::replace(child_id.get_node_mut(ctx.tree_mut()).unwrap(), old_child_node);

  // Revert the original node to its original state to apply the class.
  *orig_id.get_node_mut(ctx.tree_mut()).unwrap() = Box::new(orig.clone());
  let new_id = ctx.build(class.apply_style(Widget::from_id(orig_id)));

  let tree = ctx.tree_mut();

  new_id.wrap_node(tree, |render| {
    node.replace_data(render);
    class_node
  });

  if new_id != child_id {
    // If a pipe widget generates a widget with a class, we place the pipe node
    // outside of the class node. However, since its widget ID is altered, we must
    // notify the pipe node accordingly.
    let old_rg = child_id..=orig_id;
    let new_rg = new_id..=orig_id;
    new_id
      .query_all_iter::<PipeNode>(tree)
      .for_each(|node| {
        node
          .dyn_info_mut()
          .single_range_replace(&old_rg, &new_rg)
      });
    child_holder.replace(new_id, tree);
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

  node.dyn_info_mut().gen_range = GenRange::Single(new_id);
  let marker = tree.dirty_marker();
  marker.mark(new_id, DirtyPhase::Layout);
  if new_id != orig_id && new_id.ancestor_of(orig_id, tree) {
    marker.mark(orig_id, DirtyPhase::Layout);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};
  class_names!(MARGIN, BOX_200, CLAMP_50, EMPTY);
  use smallvec::smallvec;

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
    classes.insert(
      CLAMP_50,
      style_class! {
        clamp: BoxClamp::fixed_size(Size::new(50., 50.))
      },
    );
    classes
  }

  impl Classes {
    fn into_provider(self) -> Provider { Provider::new(self) }
  }

  #[test]
  fn switch_class() {
    reset_test_env!();

    let (cls, w_cls) = split_value(MARGIN);
    let mut wnd = TestWindow::new(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      }
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
      @Providers {
        providers: smallvec![classes.into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      }
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
      @Providers {
        providers: smallvec![classes.into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      }
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
        Providers::new([Provider::new(0i32)])
          .with_child(fn_widget! { w })
      });

      @Providers {
        providers: smallvec![classes.into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!($trigger; PROVIDER_CLS),
          on_performed_layout: |e| {
            panic!("{}", *Provider::of::<i32>(e).unwrap());
          }
        }
      }
    });
    wnd.draw_frame();
  }

  #[test]
  fn fix_not_mounted_class_node() {
    reset_test_env!();

    let (cls, w_cls) = split_value(EMPTY);
    let mut wnd = TestWindow::new(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      }
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
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      }
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
      @Providers {
        providers: smallvec![
          initd_classes().into_provider(),
          Class::provider(MARGIN, style_class!{
            clamp: BoxClamp::fixed_size(Size::new(66., 66.))
          })
        ],
        @Container {
          size: Size::new(100., 100.),
          class: MARGIN,
        }
      }
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
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @ {
          let w = pipe!(*$w_trigger).map(|_|{
            @Container {size: Size::new(100., 100.) }
          });
          @Class {
            class: pipe!(*$cls),
            @ { w }
          }
        }
      }
    });

    wnd.draw_frame();
    *w.write() = false;
    wnd.draw_frame();
    *w_cls.write() = MARGIN;
    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(120.));
  }

  #[test]
  fn fix_track_id_in_new_class() {
    reset_test_env!();

    class_names!(TRACK_ID);
    let mut classes = initd_classes();
    classes.insert(TRACK_ID, |w| {
      let mut w = FatObj::new(w);
      rdl! {
        @Container {
          size: Size::new(100., 100.),
          @ $w {
            on_performed_layout: move |e| {
              let id = $w.track_id().get().unwrap();
              assert!(!id.is_dropped(e.tree()));
            }
          }
        }
      }
      .into_widget()
    });

    let (cls, w_cls) = split_value(EMPTY);

    let mut wnd = TestWindow::new(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![classes.clone().into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$cls),
        }
      }
    });

    wnd.draw_frame();
    *w_cls.write() = TRACK_ID;
    wnd.draw_frame();
  }

  #[test]
  fn fix_pipe_class_in_pipe_class() {
    reset_test_env!();

    class_names! { PIPE_CLS, INNER_PIPE_A, INNER_PIPE_B };

    let (cls, w_cls) = split_value(INNER_PIPE_A);
    let (out, w_out) = split_value(EMPTY);
    let mut wnd = TestWindow::new(fn_widget! {
      let mut classes = Classes::default();
      classes.insert(PIPE_CLS, |w| {
        FatObj::new(w)
          .class(Variant::<ClassName>::new(BuildCtx::get()).unwrap())
          .into_widget()
      });

      let out = out.clone_watcher();
      let cls = cls.clone_watcher();
      providers!{
        providers: smallvec![
          classes.clone().into_provider(),
          Provider::value_of_watcher(cls.clone_watcher())
        ],
        @MockBox {
          class: pipe!(*$out),
          size: Size::new(100., 100.),
        }
      }
    });
    *w_out.write() = PIPE_CLS;
    wnd.draw_frame();
    *w_cls.write() = INNER_PIPE_B;
    wnd.draw_frame();
  }
}
