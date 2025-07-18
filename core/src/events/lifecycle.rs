use super::*;

/// The event fired when the widget is mounted, performed layout or disposed.
pub type LifecycleEvent = CommonEvent;

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[test]
  fn full_lifecycle() {
    reset_test_env!();

    let trigger = Stateful::new(0);
    let lifecycle = Stateful::new(vec![]);
    let c_lc = lifecycle.clone_reader();
    let c_trigger = trigger.clone_writer();
    let (is_empty, clean_trigger) = split_value(false);

    let w = fn_widget! {
      @MockBox {
        size: Size::zero(),
        @ {
          pipe!(*$read(is_empty)).map(move |v| {
            (!v).then(move || fn_widget!{
              @MockBox {
                size: Size::zero(),
                on_mounted: move |_| $write(lifecycle).push("static mounted"),
                on_performed_layout: move |_| $write(lifecycle).push("static performed layout"),
                on_disposed: move |_| $write(lifecycle).push("static disposed"),
                @ {
                  pipe!(*$read(trigger)).map(move |_| fn_widget!{
                    @MockBox {
                      size: Size::zero(),
                      on_mounted: move |_| $write(lifecycle).push("dyn mounted"),
                      on_performed_layout: move |_| $write(lifecycle).push("dyn performed layout"),
                      on_disposed: move |_| $write(lifecycle).push("dyn disposed")
                    }
                  })
                }
              }
            })
          })
        }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    assert_eq!(&**c_lc.read(), ["static mounted", "dyn mounted",]);

    wnd.draw_frame();

    assert_eq!(
      &**c_lc.read(),
      ["static mounted", "dyn mounted", "dyn performed layout", "static performed layout",]
    );
    {
      *c_trigger.write() += 1;
    }
    wnd.draw_frame();
    assert_eq!(
      &**c_lc.read(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "dyn disposed",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
      ]
    );

    {
      *clean_trigger.write() = true;
    }
    wnd.draw_frame();
    assert_eq!(
      &**c_lc.read(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "dyn disposed",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "static disposed",
        "dyn disposed"
      ]
    );
  }

  #[test]
  fn track_lifecycle() {
    reset_test_env!();

    let cnt = Stateful::new(3);
    let mounted: Stateful<HashSet<WidgetId>> = Stateful::new(HashSet::default());
    let disposed: Stateful<HashSet<WidgetId>> = Stateful::new(HashSet::default());

    let c_cnt = cnt.clone_writer();
    let c_mounted = mounted.clone_reader();
    let c_disposed = disposed.clone_reader();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$read(cnt)).map(move |cnt| {
            (0..cnt).map(move |_| {
              @MockBox {
                size: Size::zero(),
                on_mounted: move |e| { $write(mounted).insert(e.id); },
                on_disposed: move |e| { $write(disposed).insert(e.id); },
              }
            })
          })
        }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();
    let mounted_ids = c_mounted.read().clone();

    *c_cnt.write() = 5;
    wnd.draw_frame();

    assert_eq!(mounted_ids.len(), 3);
    assert_eq!(&mounted_ids, &*c_disposed.read());
  }
}
