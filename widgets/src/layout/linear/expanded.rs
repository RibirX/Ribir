use ribir_core::prelude::*;
use smallvec::SmallVec;

/// A widget that expands a child within a `Flex` container, allowing the child
/// to fill the available space. If multiple children are expanded, the
/// available space is divided among them based on their flex factor.
#[derive(Clone, Copy, PartialEq)]
// `Expand` should not support `FatObj`, as this may cause the `Expanded` widget
// to become invisible to its parent. For example, `@Expanded { margin:
// EdgeInsets::all(10.) }` actually expands as `@Margin { @Expanded { .. } }`.
pub struct Expanded {
  /// The flex factor determining how much space this child should occupy
  /// relative to other expanded children.
  pub flex: f32,

  /// Determines if the child widget should defer its space allocation until
  /// after other widgets have been allocated their space. When set to `true`,
  /// this expanded widget will not allocate any space initially and will
  /// ignore its own size constraints. Instead, it will wait for all other
  /// widgets to be allocated space first, and then divide the remaining
  /// available space based on its flex factor.
  ///
  /// The default value is `true`.
  pub defer_alloc: bool,
}

/// Macro used to generate a function widget using `Expanded` as the root
/// widget.
#[macro_export]
macro_rules! expanded {
  ($($t: tt)*) => { fn_widget! { @Expanded { $($t)* } } };
}
pub use expanded;

impl Default for Expanded {
  fn default() -> Self { Self { flex: 1., defer_alloc: false } }
}

#[derive(Default)]
pub struct ExpandedDeclarer {
  flex: Option<PipeValue<f32>>,
  defer_alloc: Option<PipeValue<bool>>,
}

impl ExpandedDeclarer {
  #[track_caller]
  pub fn with_flex<K: ?Sized>(&mut self, flex: impl RInto<PipeValue<f32>, K>) -> &mut Self {
    assert!(self.flex.is_none(), "`flex` is already set");
    self.flex = Some(flex.r_into());
    self
  }

  #[track_caller]
  pub fn with_defer_alloc<K: ?Sized>(
    &mut self, defer_alloc: impl RInto<PipeValue<bool>, K>,
  ) -> &mut Self {
    assert!(self.defer_alloc.is_none(), "`defer_alloc` is already set");
    self.defer_alloc = Some(defer_alloc.r_into());
    self
  }
}

impl Declare for Expanded {
  type Builder = ExpandedDeclarer;

  fn declarer() -> Self::Builder { ExpandedDeclarer::default() }
}

impl ObjDeclarer for ExpandedDeclarer {
  type Target = DeclarerWithSubscription<State<Expanded>>;

  fn finish(self) -> Self::Target {
    let (flex, u_flex) = self.flex.map_or((1., None), |v| v.unzip());
    let (defer_alloc, u_defer_alloc) = self
      .defer_alloc
      .map_or((true, None), |v| v.unzip());

    let host = State::value(Expanded { flex, defer_alloc });
    let mut subscribes = SmallVec::new();
    if let Some(o) = u_flex {
      let host = host.clone_writer();
      let u = o.subscribe(move |v| host.write().flex = v);
      subscribes.push(u)
    }

    if let Some(o) = u_defer_alloc {
      let host = host.clone_writer();
      let u = o.subscribe(move |v| host.write().defer_alloc = v);
      subscribes.push(u)
    }

    DeclarerWithSubscription::new(host, subscribes)
  }
}

impl<'c> ComposeChild<'c> for Expanded {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, mut child: Self::Child) -> Widget<'c> {
    let data: Box<dyn Query> = match this.try_into_value() {
      Ok(this) => Box::new(Queryable(this)),
      Err(this) => {
        child = child.dirty_on(this.raw_modifies(), DirtyPhase::Layout);
        Box::new(this)
      }
    };

    child.attach_data(data)
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::{reset_test_env, test_helper::*};
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  widget_layout_test!(
    one_line_expanded,
    WidgetTester::new(fn_widget! {
      let size = Size::new(100., 50.);
      @Flex  {
        @Expanded {
          flex: 1.,
          @SizedBox { size }
        }
        @SizedBox { size }
        @SizedBox { size }
        @Expanded {
          flex: 2.,
          @SizedBox { size }
        }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(500., 50.)),
    LayoutCase::new(&[0, 0]).with_size(Size::new(100., 50.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(100., 0., 100., 50.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(200., 0., 100., 50.)),
    LayoutCase::new(&[0, 3]).with_rect(ribir_geom::rect(300., 0., 200., 50.))
  );

  widget_layout_test!(
    wrap_expanded,
    WidgetTester::new(fn_widget! {
      let size = Size::new(100., 50.);
      @Flex {
        wrap: true,
        @Expanded {
          defer_alloc: false,
          flex: 1. ,
          @SizedBox { size }
        }
        @SizedBox { size }
        @SizedBox { size }
        @SizedBox { size }
        @SizedBox { size }
        @Expanded {
          defer_alloc: false,
          flex: 1. ,
          @SizedBox { size, }
        }
        @Expanded {
          defer_alloc: false,
          flex: 4.,
          @SizedBox { size, }
        }
      }
    })
    .with_wnd_size(Size::new(350., 500.)),
    LayoutCase::default().with_rect(ribir_geom::rect(0., 0., 350., 150.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 150., 50.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(150., 0., 100., 50.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(250., 0., 100., 50.)),
    LayoutCase::new(&[0, 3]).with_rect(ribir_geom::rect(0., 50., 100., 50.)),
    LayoutCase::new(&[0, 4]).with_rect(ribir_geom::rect(100., 50., 100., 50.)),
    LayoutCase::new(&[0, 5]).with_rect(ribir_geom::rect(200., 50., 150., 50.)),
    LayoutCase::new(&[0, 6]).with_rect(ribir_geom::rect(0., 100., 350., 50.))
  );

  #[test]
  fn modifies_flex() {
    reset_test_env!();

    let (flex, w_flex) = split_value(1f32);
    let widget = fn_widget! {
      let expanded = @Expanded { flex: 1. };
      watch!(*$read(flex)).subscribe(move |val| $write(expanded).flex = val);

      @Flex {
        h_align: HAlign::Stretch,
        @(expanded) { @ { Void } }
        @Expanded {
          flex: 1.,
          @ { Void }
        }
        @SizedBox { size: Size::new(100., 100.) }
      }
    };

    let wnd = TestWindow::new_with_size(widget, Size::new(400., 100.));
    wnd.draw_frame();
    LayoutCase::expect_size(&wnd, &[0, 0], Size::new(150., 0.));
    *w_flex.write() = 2.;
    wnd.draw_frame();
    LayoutCase::expect_size(&wnd, &[0, 0], Size::new(200., 0.));
  }
}
