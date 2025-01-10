use ribir_core::prelude::*;

use crate::layout::Stack;

/// This widget wraps its child in a `ScrollableWidget` and adds two scrollbar
/// for interactivity and visual scroll position indication.
///
/// The visibility of the thumb is determined by the scrollable of its axis.
/// For instance, the vertical scrollbar is displayed only when the child's
/// height exceeds its container's height, and the `ScrollableWidget` is set to
/// be scrollable in the vertical direction. By default, the scrollbar is
/// enabled for `Scrollable::Y`, but you can utilize
/// `Scrollbar::inner_scrollable_widget` to access the `ScrollableWidget` and
/// switch between which scrollbar to enable.
///
/// `Scrollbar` offers five class names for users or themes to customize the
/// scrollbar appearance. The `Scrollbar` positions the scrollbar on the
/// scrollable child widget, and adjusting the scrollbar's placement (left,
/// right, top, or bottom) depends on the class names' implementation.
///
/// `Scrollbar` also provides the inner `ScrollableWidget` through the
/// `Provider`, accessible in any descendants of the scrollbar. For instance,
/// when implementing the class name, you can utilize
/// `Provider::of::<ScrollableWidget>` to retrieve the scroll status and
/// determine the scrollbar's appearance.

#[derive(Declare)]
pub struct Scrollbar {
  #[declare(default=Stateful::new(ScrollableWidget::default()))]
  scroll: Stateful<ScrollableWidget>,
}

class_names! {
  #[doc = "Class name for the thumb of the horizontal scrollbar"]
  H_SCROLL_THUMB,
  #[doc = "Class name for the track of the horizontal scrollbar"]
  H_SCROLL_TRACK,
  #[doc = "Class name for the thumb of the vertical scrollbar"]
  V_SCROLL_THUMB,
  #[doc = "Class name for the track of the vertical scrollbar"]
  V_SCROLL_TRACK,
  #[doc = "Class name for the scrollable widget of the scrollbar"]
  SCROLL_CLIENT_AREA
}

impl Scrollbar {
  pub fn new(scrollable: Scrollable) -> Self {
    let mut inner = ScrollableWidget::default();
    inner.scrollable = scrollable;
    Self { scroll: Stateful::new(inner) }
  }

  /// Return the `ScrollableWidget` of the scrollbar. You can utilize it to
  /// scroll the child or access scroll information.
  pub fn inner_scrollable_widget(&self) -> &Stateful<ScrollableWidget> { &self.scroll }
}

impl<'c> ComposeChild<'c> for Scrollbar {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let scroll = this.read().scroll.clone_writer();
    // Here we provide the `ScrollableWidget`, which allows the theme to access
    // scroll states or enables descendants to trigger scrolling to a different
    // position.
    providers! {
      providers: smallvec::smallvec![
        Provider::new(scroll.clone_writer()),
        Provider::value_of_writer(scroll.clone_writer(), None),
      ],
      @ {
        let h_scrollbar = distinct_pipe!($scroll.is_x_scrollable())
          .map(move |need_bar| need_bar.then(||{
            let mut h_track = @Stack {
              class: H_SCROLL_TRACK,
              clamp: BoxClamp::EXPAND_X,
              on_wheel: move |e| $scroll.write().scroll(-e.delta_x, -e.delta_y),
            };

            @ $h_track {
              on_tap: move |e| if e.is_primary {
                let rate = e.position().x / $h_track.layout_width();
                let mut scroll = $scroll.write();
                let x = rate * scroll.max_scrollable().x;
                let scroll_pos = Point::new(x, scroll.get_scroll_pos().y);
                scroll.jump_to(scroll_pos);
              },
              @Container {
                class: H_SCROLL_THUMB,
                size: distinct_pipe!{
                  let width = h_thumb_rate(&$scroll) * $h_track.layout_width();
                  Size::new(width, 0.)
                },
                anchor: distinct_pipe!{
                  let pos = $scroll.get_x_scroll_rate() * $h_track.layout_width();
                  Anchor::left(pos)
                },
              }
            }
          }));

        let v_scrollbar = distinct_pipe!($scroll.is_y_scrollable())
          .map(move |need_bar| need_bar.then(|| {
            let mut v_track = @Stack {
              class: V_SCROLL_TRACK,
              clamp: BoxClamp::EXPAND_Y,
              on_wheel: move |e| $scroll.write().scroll(-e.delta_x, -e.delta_y),
            };

            @ $v_track {
              on_tap: move |e| if e.is_primary {
                let rate = e.position().y / $v_track.layout_height();
                let mut scroll = $scroll.write();
                let y = rate * scroll.max_scrollable().y;
                let scroll_pos = Point::new(scroll.get_scroll_pos().x, y);
                scroll.jump_to(scroll_pos);
              },
              @Container {
                class: V_SCROLL_THUMB,
                size: distinct_pipe!{
                  let height = v_thumb_rate(&$scroll) * $v_track.layout_height();
                  Size::new(0., height)
                },
                anchor: distinct_pipe!{
                  let pos = $scroll.get_y_scroll_rate() * $v_track.layout_height();
                  Anchor::top(pos)
                },
              }
            }
          }));

        let scroll = FatObj::new(scroll);
        @Stack {
          @ $scroll {
            class: SCROLL_CLIENT_AREA,
            @{ child }
          }
          @ { h_scrollbar }
          @ { v_scrollbar }
        }
      }
    }
    .into_widget()
  }
}

fn h_thumb_rate(s: &ScrollableWidget) -> f32 {
  s.scroll_view_size().width / s.scroll_content_size().width
}
fn v_thumb_rate(s: &ScrollableWidget) -> f32 {
  s.scroll_view_size().height / s.scroll_content_size().height
}

#[cfg(test)]
mod test {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  widget_test_suit!(
    init,
    WidgetTester::new(fn_widget! {
      let scrollbar = Scrollbar::new(Scrollable::Both);
      @ $scrollbar {
        @Container { size: Size::new(500., 500.) }
      }
    })
    .with_wnd_size(Size::new(100., 100.)),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_test_suit!(
    scrolled,
    {
      let mut inner = ScrollableWidget::default();
      inner.scrollable = Scrollable::Both;
      let inner = Stateful::new(inner);
      let inner2 = inner.clone_writer();

      WidgetTester::new(fn_widget! {
        let scrollbar = Scrollbar { scroll : inner.clone_writer() };
        @ $scrollbar {
          @Container { size: Size::new(500., 500.) }
        }
      })
      .with_wnd_size(Size::new(100., 100.))
      .on_initd(move |wnd| {
        // Trigger a frame before scrolling to ensure the scrollbar is generated by the
        // pipe.
        wnd.draw_frame();
        inner2.write().scroll(50., 50.);
      })
    },
    LayoutCase::default().with_size(Size::new(100., 100.))
  );
}
