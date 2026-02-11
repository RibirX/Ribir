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
//! let theme_fn = move || {
//!   let mut theme = Theme::default();
//!   // Define how `RED_BORDER` transforms a widget.
//!   theme.classes.insert(
//!     RED_BORDER,
//!     style_class! {
//!       border: Border::all(BorderSide::new(2., Color::RED.into()))
//!     },
//!   );
//!   theme
//! };
//!
//! let w = fn_widget! {
//!   @Container {
//!     size: Size::new(100., 100.),
//!     class: RED_BORDER,
//!   }
//! };
//!
//! App::run(w).with_app_theme(theme_fn);
//! ```

use std::hash::Hash;

use pipe::PipeNode;
use smallvec::{SmallVec, smallvec};

use crate::{pipe::GenRange, prelude::*, ticker::FrameMsg, window::WindowId};

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
      rdl! {
        @FatObj {
          $( $field: $value, )*
          @ { widget }
        }
      }.into_widget()
    }) as $crate::prelude::ClassImpl
  };
}

/// This macro generates a function for creating a styled widget with predefined
/// fields. It simplifies the process of applying consistent styles to widgets.
///
/// # Usage
/// ```rust
/// use ribir::prelude::*;
///
/// named_style_impl!(primary_button => {
///   padding: EdgeInsets::all(8.),
///   background: Color::BLUE,
/// });
/// ```
#[macro_export]
macro_rules! named_style_impl {
  ($(#[$meta:meta])* $style_name:ident => {
      $($field:ident: $value:expr),* $(,)?
  }) => {
    $(#[$meta])*
    fn $style_name(widget: $crate::prelude::Widget) -> $crate::prelude::Widget {
      rdl! {
        @FatObj {
          $( $field: $value, )*
          @ { widget }
        }.into_widget()
      }
    }
  };
}

/// This macro generates multiple styled widget builder functions in one go.
/// It helps in defining several styles simultaneously, reducing repetition.
///
/// # Example
/// ```
/// use ribir::prelude::*;
///
/// named_styles_impl! {
///   /// Secondary button style for auxiliary actions
///   secondary_button => {
///       padding: EdgeInsets::all(6.),
///       background: Color::GRAY,
///   },
///
///   /// Danger button style for destructive operations
///   danger_button => {
///       padding: EdgeInsets::all(8.),
///       background: Color::RED,
///   }
/// }
/// ```
#[macro_export]
macro_rules! named_styles_impl {
  ($( $(#[$meta:meta])* $name:ident => { $($field:ident: $value:expr),* $(,)? } ),* $(,)? ) => {
    $(
      named_style_impl! {
        $(#[$meta])*
        $name => { $($field: $value),* }
      }
    )*
  };
}

/// Combines multiple class implementations into a single implementation.
/// This macro takes a list of class implementations and returns a closure
/// that applies each implementation sequentially to a `Widget`.
///
/// The first implementation in the list runs first, and the last one is
/// applied last (closest to the widget). Therefore, the last implementation
/// has the highest visual priority (e.g. `[BASE, SELECTED]` ->
/// `BASE(SELECTED(child))`).
///
/// # Example
/// ```
/// use ribir::prelude::*;
///
/// class_names!(PADDING_AND_BG);
///
/// fn init_classes(classes: &mut Classes) {
///   classes.insert(
///     PADDING_AND_BG,
///     class_chain_impl![
///       style_class! { padding: EdgeInsets::all(4.) },
///       style_class! { background: Color::BLUE }
///     ],
///   );
/// }
/// ```
#[macro_export]
macro_rules! class_chain_impl {
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

#[derive(Default, Clone, PartialEq, Debug)]
pub struct ClassList {
  classes: SmallVec<[ClassName; 1]>,
}

impl ClassList {
  #[inline]
  pub fn new() -> Self { Self::default() }

  #[inline]
  pub fn push(&mut self, class: ClassName) { self.classes.push(class); }

  #[inline]
  pub fn pop(&mut self) -> Option<ClassName> { self.classes.pop() }

  #[inline]
  pub fn len(&self) -> usize { self.classes.len() }

  #[inline]
  pub fn is_empty(&self) -> bool { self.classes.is_empty() }

  #[inline]
  pub fn iter(&self) -> std::slice::Iter<'_, ClassName> { self.classes.iter() }
}

impl IntoIterator for ClassList {
  type Item = ClassName;
  type IntoIter = smallvec::IntoIter<[ClassName; 1]>;

  #[inline]
  fn into_iter(self) -> Self::IntoIter { self.classes.into_iter() }
}

impl<'a> IntoIterator for &'a ClassList {
  type Item = &'a ClassName;
  type IntoIter = std::slice::Iter<'a, ClassName>;

  #[inline]
  fn into_iter(self) -> Self::IntoIter { self.classes.iter() }
}

impl FromIterator<ClassName> for ClassList {
  fn from_iter<T: IntoIterator<Item = ClassName>>(iter: T) -> Self {
    Self { classes: iter.into_iter().collect() }
  }
}

/// This widget is used to apply class to its child widget by the `ClassName`.
#[derive(Default, Clone, PartialEq)]
pub struct Class {
  pub class: ClassList,
}

/// This macro is used to generate a function widget using `Class` as the root
/// widget.
#[macro_export]
macro_rules! class {
  ($($t: tt)*) => { fn_widget! { @Class { $($t)* } } };
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
}

impl<R: StateReader<Value = Classes> + Query> ProviderSetup for ClassesReaderSetup<R> {
  fn setup(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderRestore> {
    let classes = self.0;
    let overrides = classes.read().remove_intersects_class(map);
    let classes = Box::new(Setup::from_state(classes)).setup(map);
    Box::new(ClassesRestore { overrides, classes })
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
    let f = move || {
      match this.try_into_value() {
        Ok(c) => compose_with_classes(c.class, child),
        Err(writer) => {
          // Dynamic case: the whole Class is stateful

          let ctx = BuildCtx::get();
          let dummy = GenRange::Single(ctx.tree().dummy_id());
          let cls_child = ClassNode::empty_node(dummy.clone());
          let orig_child = ClassNode::empty_node(dummy);
          let orig_child2 = orig_child.clone();
          let child = child.on_build(move |orig_id| orig_child2.init_for_single(orig_id));

          let cls_child2 = cls_child.clone();
          let orig_child2 = orig_child.clone();
          let wnd_id = ctx.window().id();
          let sampler = ctx
            .window()
            .frame_tick_stream()
            .filter(|msg| matches!(msg, FrameMsg::NewFrame(_)));

          let u = pipe!($read(writer).class.clone())
            .with_effect(ModifyEffect::FRAMEWORK)
            .into_observable()
            .distinct_until_changed()
            .skip(1)
            .sample(sampler)
            .subscribe(move |new_classes| {
              classes_update(&cls_child2, &orig_child2, new_classes, wnd_id);
            })
            .unsubscribe_when_dropped();

          compose_with_classes(writer.read().class.clone(), child).on_build(move |child_id| {
            cls_child.init_for_single(child_id);
            child_id.attach_anonymous_data(u, BuildCtx::get_mut().tree_mut());
          })
        }
      }
    };
    FnWidget::new(f).into_widget()
  }
}

/// Compose a widget with multiple classes, chaining them in reverse order.
fn compose_with_classes(classes: ClassList, child: Widget) -> Widget {
  let mut widget = child;
  for cls in classes.classes.into_iter().rev() {
    widget = apply_class(Some(cls), widget);
  }
  widget
}

fn apply_class(class: Option<ClassName>, w: Widget) -> Widget {
  if let Some(cls_impl) = class_impl(class) { cls_impl(w) } else { w }
}

fn class_impl(class: Option<ClassName>) -> Option<ClassImpl> {
  let cls = class?;
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

impl Class {
  /// Creating a provider for a class, which can either provide the
  /// implementation of a class or be used to override the implementation of a
  /// class.
  ///
  /// This is a builtin field of FatObj. You can simply set the `class` field
  /// to attach a Class widget to the host widget.
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
}

type ClassNode = PipeNode;

/// Update the class chain when the whole Class is changed.
fn classes_update(node: &ClassNode, orig: &ClassNode, classes: ClassList, wnd_id: WindowId) {
  let wnd =
    AppCtx::get_window(wnd_id).expect("This handle is not valid because the window is closed");

  let child_id = node.dyn_info().host_id();
  let orig_id = orig.dyn_info().host_id();
  if child_id.is_dropped(wnd.tree()) {
    return;
  }

  let child_holder = child_id.place_holder(wnd.tree_mut());

  let old_child_node = node.take_data();
  let _guard = BuildCtx::init_for(child_id, wnd.tree);
  let ctx = BuildCtx::get_mut();

  let class_node =
    std::mem::replace(child_id.get_node_mut(ctx.tree_mut()).unwrap(), old_child_node);

  *orig_id.get_node_mut(ctx.tree_mut()).unwrap() = Box::new(orig.clone());
  let new_id = ctx.build(compose_with_classes(classes, Widget::from_id(orig_id)));

  let tree = ctx.tree_mut();

  if child_id != new_id {
    class_node.update_track_id(new_id);
  }

  new_id.wrap_node(tree, |render| {
    node.replace_data(render);
    class_node
  });

  if new_id != child_id {
    new_id
      .query_all_iter::<PipeNode>(tree)
      .for_each(|node| node.dyn_info_mut().replace(child_id, new_id));
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

impl From<ClassName> for ClassList {
  #[inline]
  fn from(v: ClassName) -> Self { ClassList { classes: smallvec![v] } }
}

impl From<Option<ClassName>> for ClassList {
  #[inline]
  fn from(v: Option<ClassName>) -> Self {
    if let Some(v) = v { ClassList { classes: smallvec![v] } } else { ClassList::default() }
  }
}

impl<const N: usize> From<[ClassName; N]> for ClassList {
  #[inline]
  fn from(v: [ClassName; N]) -> Self { ClassList::from_iter(v) }
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
          on_mounted: |_| {println!("mounted");},
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
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
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

  fn on_disposed_of_class_nodes() {
    reset_test_env!();

    class_names!(ON_DISPOSED);

    let (cls, w_cls) = split_value(ON_DISPOSED);

    static mut DISPOSED: bool = false;

    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      let mut classes = initd_classes();
      classes.insert(ON_DISPOSED, move |w| {
        fn_widget! {
          @MockBox {
            size: Size::zero(),
              on_disposed: move |_| unsafe { DISPOSED = true },
            @ { w }
          }
        }
        .into_widget()
      });
      @Providers {
        providers: smallvec![classes.into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    assert!(unsafe { !DISPOSED });

    *w_cls.write() = MARGIN;
    wnd.draw_frame();
    assert!(unsafe { DISPOSED });
  }

  #[test]
  fn class_chain() {
    reset_test_env!();

    let wnd = TestWindow::from_widget(fn_widget! {
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: [MARGIN, CLAMP_50],
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(70., 70.));
  }

  #[test]
  fn fix_crash_for_class_impl_may_have_multi_child() {
    reset_test_env!();

    class_names!(MULTI);
    let (cls, w_cls) = split_value(MARGIN);
    let wnd = TestWindow::from_widget(fn_widget! {
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
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
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
  fn fix_provider_in_pipe_class() {
    reset_test_env!();

    class_names!(PROVIDER_CLS);

    let (r_val, w_val) = split_value(-1);
    let wnd = TestWindow::from_widget(fn_widget! {
      let trigger = Stateful::new(true);
      let mut classes = Classes::default();
      classes.insert(PROVIDER_CLS, |w| {
        Providers::new([Provider::new(0i32)])
          .with_child(fn_widget! { w })
      });

      @Providers {
        providers: smallvec![classes.into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!($read(trigger); PROVIDER_CLS),
          on_performed_layout: move |e| {
            *$write(w_val) =  *Provider::of::<i32>(e).unwrap();
          }
        }
      }
    });
    wnd.draw_frame();

    assert_eq!(*r_val.read(), 0);
  }

  #[test]
  fn fix_not_mounted_class_node() {
    reset_test_env!();

    let (cls, w_cls) = split_value(EMPTY);
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
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
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
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

    let wnd = TestWindow::from_widget(fn_widget! {
      @Providers {
        providers: smallvec![
          initd_classes().into_provider(),
          Class::provider(MARGIN, style_class!{
            clamp: BoxClamp::fixed_size(Size::new(66., 66.))
          })
        ],
        @Container {
          hint_size: Size::new(100., 100.),
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

    let wnd = TestWindow::from_widget(fn_widget! {
      let w_trigger = w_trigger.clone_watcher();
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @ {
          let w = pipe!(*$read(w_trigger)).map(|_| fn_widget!{
            @Container {hint_size: Size::new(100., 100.) }
          });
          @Class {
            class: pipe!(*$read(cls)),
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
          hint_size: Size::new(100., 100.),
          @(w) {
            on_performed_layout: move |e| {
              let id = $clone(w.track_id()).get().unwrap();
              assert!(!id.is_dropped(e.tree()));
            }
          }
        }
      }
      .into_widget()
    });

    let (cls, w_cls) = split_value(EMPTY);

    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![classes.clone().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
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
    let wnd = TestWindow::from_widget(fn_widget! {
      let mut classes = Classes::default();
      classes.insert(PIPE_CLS, style_class!{
        class: Variant::<ClassName>::new(BuildCtx::get()).unwrap()
      });

      let out = out.clone_watcher();
      let cls = cls.clone_watcher();
      providers!{
        providers: smallvec![
          classes.clone().into_provider(),
          Provider::watcher(cls.clone_watcher())
        ],
        @MockBox {
          class: pipe!(*$read(out)),
          size: Size::new(100., 100.),
        }
      }
    });
    *w_out.write() = PIPE_CLS;
    wnd.draw_frame();
    *w_cls.write() = INNER_PIPE_B;
    wnd.draw_frame();
  }

  #[test]
  fn fix_pipe_class_unsubscribed() {
    reset_test_env!();

    class_names! { OUT_PIPE_CLS, OUT_PIPE_CLS_2, INNER_PIPE, INNER_PIPE_2};

    let inner_apply = Stateful::new(0usize);
    let w_inner_apply = inner_apply.clone_writer();
    let (inner, w_inner) = split_value(false);
    let (out, w_out) = split_value(OUT_PIPE_CLS);
    let wnd = TestWindow::from_widget(fn_widget! {
      let out_cls = Class::provider(OUT_PIPE_CLS, style_class!{
        class: Variant::<bool>::new(BuildCtx::get()).unwrap()
          .map(|b| if *b { INNER_PIPE } else { INNER_PIPE_2 } )
      });
      let out_cls_2 = Class::provider(OUT_PIPE_CLS_2, style_class!{
        class: Variant::<bool>::new(BuildCtx::get()).unwrap()
          .map(|b| if *b { INNER_PIPE } else { INNER_PIPE_2 } )
      });
      let inner_cls = Class::provider(INNER_PIPE, |w| {
        *Provider::write_of::<usize>(BuildCtx::get()).unwrap() += 1;
        w
      });
      let inner_cls_2 = Class::provider(INNER_PIPE_2, |w| {
        *Provider::write_of::<usize>(BuildCtx::get()).unwrap() += 1;
        w
      });

      let out = out.clone_watcher();
      let inner = inner.clone_watcher();
      let w_inner_apply = w_inner_apply.clone_writer();
      providers!{
        providers: smallvec![
          out_cls, out_cls_2, inner_cls, inner_cls_2,
          Provider::watcher(inner.clone_watcher()),
          Provider::writer(w_inner_apply.clone_writer(), None),
        ],
        @MockBox {
          class: pipe!(*$read(out)),
          size: Size::new(100., 100.),
        }
      }
    });
    wnd.draw_frame();
    assert_eq!(*inner_apply.read(), 1);

    *w_out.write() = OUT_PIPE_CLS_2;
    wnd.draw_frame();
    assert_eq!(*inner_apply.read(), 2);

    *w_inner.write() = true;
    wnd.draw_frame();
    assert_eq!(*inner_apply.read(), 3);
  }

  // the track_id is bind after the class, when the class is changed and wrap with
  // new reader(here is the margin), the track_id should changed.
  #[test]
  fn fix_track_id_in_class_node() {
    reset_test_env!();

    class_names! { WRAP_CLS, IDENTITY_CLS };

    let (r_cls, w_cls) = split_value(IDENTITY_CLS);
    let (r_id, w_id) = split_value(None);
    let w = fn_widget! {
      let cls = Class::provider(WRAP_CLS, style_class!(
        margin: EdgeInsets::all(2.),
      ));

      let mut w = FatObj::new(
        @Void {
          class: pipe!(*$read(r_cls)),
        }.into_widget()
      );
      *$write(w_id) = Some($clone(w.track_id()));

      @Providers{
        providers: smallvec![
          cls,
        ],
        @ { w }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    let id1 = r_id.read().as_ref().and_then(|w| w.get());

    *w_cls.write() = WRAP_CLS;
    wnd.draw_frame();

    let id2 = r_id.read().as_ref().and_then(|w| w.get());
    assert!(id1 != id2);
    assert!(!id2.unwrap().is_dropped(wnd.tree()));
  }

  #[test]
  fn override_size_by_widget_field() {
    reset_test_env!();

    let mut classes = Classes::default();
    classes.insert(
      CLAMP_50,
      style_class! {
        clamp: BoxClamp::fixed_size(Size::new(50., 50.))
      },
    );

    let cls = Stateful::new(Some(CLAMP_50));

    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_writer();
      @Providers {
        providers: smallvec![classes.clone().into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(100., 100.));
  }

  #[test]
  fn fix_pipe_parent_with_pipe_class() {
    reset_test_env!();

    class_names!(CLS_A, CLS_B);

    let (expanded, w_expanded) = split_value(true);
    let (cls_toggle, w_cls) = split_value(true);

    let w = fn_widget! {
      let mut classes = Classes::default();
      // CLS_A and CLS_B use margin to create real wrapper nodes,
      // matching gallery behavior (RAIL_ITEM_SELECTED/UNSELECTED use style_class with real props).
      classes.insert(CLS_A, style_class! { margin: EdgeInsets::all(1.) });
      classes.insert(CLS_B, style_class! { margin: EdgeInsets::all(2.) });

      // Pipe parent: switches between MockMulti (horizontal) and MockStack (overlap)
      let pipe_parent = pipe!(*$read(expanded)).map(move |is_horiz| {
        if is_horiz {
          MockMulti.into_multi_child()
        } else {
          MockStack {}.into_multi_child()
        }
      }).into_multi_child();

      // Pipe class
      let cls = pipe!(*$read(cls_toggle)).map(|v| {
        if v { CLS_A } else { CLS_B }
      });

      let mut obj = FatObj::new(pipe_parent);
      obj.with_class(cls);

      @Providers {
        providers: smallvec::smallvec![Provider::new(classes)],
        @(obj) {
          @MockBox { size: Size::new(10., 10.) }
          @MockBox { size: Size::new(10., 10.) }
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    // MockMulti + margin 1 on each side: width=20+2, height=10+2
    wnd.assert_root_size(Size::new(22., 12.));

    // Toggle pipe parent: MockMulti -> MockStack
    *w_expanded.write() = false;
    wnd.draw_frame();
    // MockStack + margin 1: width=10+2, height=10+2
    wnd.assert_root_size(Size::new(12., 12.));

    // Toggle class: CLS_A -> CLS_B (margin 1 -> margin 2)
    *w_cls.write() = false;
    wnd.draw_frame();
    // MockStack + margin 2: width=10+4, height=10+4
    wnd.assert_root_size(Size::new(14., 14.));

    // Toggle pipe parent back: MockStack -> MockMulti
    *w_expanded.write() = true;
    wnd.draw_frame();
    // MockMulti + margin 2: width=20+4, height=10+4
    wnd.assert_root_size(Size::new(24., 14.));

    // Simultaneous: toggle both class and parent on the same frame
    *w_cls.write() = true; // CLS_B -> CLS_A (margin 2 -> margin 1)
    *w_expanded.write() = false; // MockMulti -> MockStack
    wnd.draw_frame();
    // MockStack + margin 1: width=10+2, height=10+2
    wnd.assert_root_size(Size::new(12., 12.));
  }
}
