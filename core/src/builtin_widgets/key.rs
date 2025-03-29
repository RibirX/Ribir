use std::{cmp::Eq, fmt::Debug};

use crate::prelude::*;

/// `Key` help `Ribir` to track if two widget is a same widget in two frames.
/// Abstract all builtin key into a same type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
  Number(isize),
  String(CowArc<str>),
  Pointer(*const ()),
}

impl<T: ?Sized> From<*const T> for Key {
  fn from(p: *const T) -> Self { Key::Pointer(p as *const ()) }
}

impl From<isize> for Key {
  fn from(value: isize) -> Self { Key::Number(value) }
}

impl From<String> for Key {
  fn from(s: String) -> Self { Key::String(s.into()) }
}

impl From<&str> for Key {
  fn from(s: &str) -> Self { Key::String(s.to_string().into()) }
}

/// `KeyWidget` is designed to track widgets during the pipe regeneration
/// process. It can only be utilized when a pipe returns multiple widgets.
///
/// When the pipe regenerates widgets, if a newly - generated widget has the
/// same key as an old one, the old widget will be reused. Otherwise, the new
/// widget will be employed. `Note` that the comparison of `KeyWidget` is
/// carried out within the same pipe.
///
/// ## Example
/// ```rust no_run
/// # use ribir::prelude::*;
/// #[derive(Clone)]
/// struct Item {
///   id: usize,
/// }
/// impl Item {
///   fn into_lazy_widget(self) -> Widget<'static> {
///     fn_widget! { @Text { text: self.id.to_string() } }.into_widget()
///   }
/// }
/// let w = fn_widget! {
///   let id_gen = Stateful::new(0);
///   let items = Stateful::new(vec![]);
///   @Column {
///     @FilledButton{
///       on_tap: move |_| {
///         $items.write().push(Item { id: *$id_gen });
///         *$id_gen.write() += 1;
///       },
///       @ { "add item"}
///     }
///     @ { pipe!($items;).map(move |_| {
///         move || {
///           $items.clone().into_iter().map(move |item| {
///             @KeyWidget {
///               key: item.id as isize,
///               @ { item.into_lazy_widget() }
///             }
///           })
///         }
///       })
///     }
///   }
/// };
/// App::run(w);
/// ```
#[simple_declare(stateless)]
pub struct KeyWidget {
  pub(crate) key: Key,
}

impl KeyWidget {
  pub fn with_child<'c, const M: usize, W>(self, child: W) -> (Key, Widget<'c>)
  where
    W: IntoWidget<'c, M>,
  {
    (self.key, child.into_widget())
  }
}

#[cfg(test)]
mod tests {
  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn key_widget() {
    reset_test_env!();

    struct IdGen {
      id: usize,
    }
    impl IdGen {
      fn new() -> Self { Self { id: 0 } }
      fn next(&mut self) -> usize {
        let id = self.id;
        self.id += 1;
        id
      }
    }

    let mut id_gen = IdGen::new();
    let (items_reader, items) = split_value(vec![]);
    let (mounts_id, mounts) = split_value(vec![]);
    items.write().push(id_gen.next());
    items.write().push(id_gen.next());
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!($items_reader;).map(move |_| {
            move || {
              $items_reader.clone().into_iter().map(move |id| {
                @KeyWidget {
                  key: id as isize,
                  @Void {
                    on_mounted: move |_| {$mounts.write().push(id);}
                  }
                }
              })
            }
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    assert_eq!(*mounts_id.read(), vec![0, 1]);

    items.write().push(id_gen.next());
    wnd.draw_frame();
    assert_eq!(*mounts_id.read(), vec![0, 1, 2]);

    items.write().remove(1);
    wnd.draw_frame();
    assert_eq!(*mounts_id.read(), vec![0, 1, 2]);
  }
}
