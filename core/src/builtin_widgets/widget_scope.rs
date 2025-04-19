use std::{hash::Hash, ops::DerefMut};

use smallvec::smallvec;

use crate::prelude::*;

/// WidgetScope
///
/// A scope that caches widgets based on their keys.
/// You should use [`GlobalWidgets`] or [`LocalWidgets`] directly.
pub struct WidgetScope<K>
where
  K: Eq + Hash + Clone,
{
  widgets: ahash::HashMap<K, Reusable>,
}

impl<K> WidgetScope<K>
where
  K: Eq + Hash + Clone + 'static,
{
  /// Returns a widget associated with the given key.
  pub fn get(&self, key: &K) -> Option<Widget<'static>> {
    self.widgets.get(key).map(move |r| r.get_widget())
  }

  /// Checks if a widget associated with the given key is currently in use.
  pub fn is_in_used(&self, key: &K) -> bool {
    self
      .widgets
      .get(key)
      .is_some_and(|r| r.is_in_used())
  }

  /// Inserts a reusable widget into the cache.
  pub fn insert_reusable(&mut self, key: K, reusable: Reusable) {
    self.widgets.insert(key, reusable);
  }

  /// Removes a widget associated with the given key from the cache.
  pub fn remove(&mut self, key: &K) { self.widgets.remove(key); }

  /// Removes all widgets from the cache.
  pub fn clear(&mut self) { self.widgets.clear(); }

  /// Returns an iterator over the ids of the cached widgets.
  pub fn get_ids(&self) -> impl Iterator<Item = K> + '_ { self.widgets.keys().cloned() }

  fn new() -> Self { Self { widgets: ahash::HashMap::default() } }
}

pub(crate) fn get_or_insert<'a, K>(
  this: &impl StateWriter<Value = impl DerefMut<Target = WidgetScope<K>>>, key: &K,
  widget: Widget<'a>,
) -> Option<Widget<'a>>
where
  K: Eq + Hash + Clone + 'static,
{
  let w = this.write().get(key);
  if w.is_none() {
    let this = this.clone_writer();
    let (w, reusable) = Reusable::new(widget);
    let key = key.clone();
    return Some(w.into_widget().on_build(move |_| {
      this.write().insert_reusable(key, reusable);
    }));
  }
  w
}

/// GlobalWidgets
///
/// A global scope that manages the lifecycle of global Widget instances in the
/// same window.
///
/// The `GlobalWidgets` provides widget caching with the following behavior:
/// - Widgets are stored globally and can be accessed from anywhere in the
///   window
/// - There is no automatic disposal - widgets must be explicitly removed
/// - Widget GlobalId must be unique across the entire window
///
/// The global cache widget can be accessed either via the built in reuse_id
/// field with a GlobalId or directly through the get method of GlobalWidgets.
pub struct GlobalWidgets(WidgetScope<GlobalId>);
impl Default for GlobalWidgets {
  fn default() -> Self { Self(WidgetScope::new()) }
}

/// LocalWidgets
///
/// A scope that manages the lifecycle of [`LocalWidget`] instances. Each
/// `LocalWidgets`:
/// - Maintains a cache of widgets identified by unique keys
/// - Automatically removes unused widgets when they're disposed (unless
///   configured DisposePolicy::Always)
///
/// Widget management behavior:
/// - When a widget is requested via `get()`:
///   - If the key exists in cache and the widget hasn't been disposed, the
///     cached instance is returned
///   - If no cached widget exists and a builder is provided, the builder is
///     invoked to create a new widget
/// - Widgets can be explicitly inserted or removed via `insert()` and
///   `remove()`
///
/// When nested within multiple `LocalWidgets`s, widget management is handled by
/// the nearest ancestor scope in the widget hierarchy.
///
/// The local cache widget can be accessed either via the built in reuse_id
/// field with a LocalId or directly through the get method of LocalWidgets.
/// Example see [`ReuseId`].
pub struct LocalWidgets(WidgetScope<LocalId>);

pub struct LocalWidgetsDeclarer {
  fat_obj: FatObj<()>,
}

impl Declare for LocalWidgets {
  type Builder = LocalWidgetsDeclarer;
  fn declarer() -> Self::Builder { LocalWidgetsDeclarer { fat_obj: FatObj::default() } }
}

impl ObjDeclarer for LocalWidgetsDeclarer {
  type Target = FatObj<LocalWidgets>;

  fn finish(self) -> Self::Target {
    self
      .fat_obj
      .map(|_| LocalWidgets(WidgetScope::new()))
  }
}

impl Deref for LocalWidgetsDeclarer {
  type Target = FatObj<()>;

  fn deref(&self) -> &Self::Target { &self.fat_obj }
}

impl DerefMut for LocalWidgetsDeclarer {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.fat_obj }
}

impl Deref for GlobalWidgets {
  type Target = WidgetScope<GlobalId>;

  fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for GlobalWidgets {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Deref for LocalWidgets {
  type Target = WidgetScope<LocalId>;

  fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for LocalWidgets {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl<'w> ComposeChild<'w> for LocalWidgets {
  type Child = Widget<'w>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'w> {
    fn_widget! {
      @Providers {
        providers: smallvec![Provider::value_of_writer(this, None)],
        @ { child }
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use super::*;
  use crate::test_helper::*;

  impl LocalWidgets {
    fn count(&self) -> usize { self.widgets.len() }
  }

  #[test]
  fn local_key() {
    reset_test_env!();
    let (build_cnt, build_w) = split_value(0);
    let (item_cnt, item_w) = split_value(1);
    let local_scope = Sc::new(RefCell::new(None));
    let local_scope2 = local_scope.clone();
    let w = fn_widget! {

      @ LocalWidgets {
        on_mounted: {
          let local_scope2 = local_scope2.clone();
          move |e| {
            *local_scope2.borrow_mut() = Some(
              Provider::state_of::<Box<dyn StateWriter<Value = LocalWidgets>>>(e)
              .unwrap()
              .clone_writer()
            );
          }
        },
        @MockMulti {
        @ {
            pipe!(*$item_cnt).map(move |cnt|
              move || {
              @ {
                (0..cnt).map(move |i| {
                  @Reuse {
                    reuse_id: LocalId::number(i),
                    @ {
                      fn_widget! {
                        *$build_w.write() += 1;
                        Void {}.into_widget()
                      }
                    }
                  }
                })
              }
            })
          }
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    let local_scope = local_scope.borrow_mut().take().unwrap();
    assert_eq!(*build_cnt.read(), 1);
    assert_eq!(local_scope.read().count(), 1);

    *item_w.write() = 4;
    wnd.draw_frame();

    assert_eq!(*build_cnt.read(), 4);
    assert_eq!(local_scope.read().count(), 4);

    *item_w.write() = 2;
    wnd.draw_frame();

    assert_eq!(*build_cnt.read(), 4);
    assert_eq!(local_scope.read().count(), 2);
  }
}
