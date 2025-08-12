use ribir_core::prelude::*;

use crate::layout::*;

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
pub struct Scrollbar {
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

/// Macro used to generate a function widget using `Scrollbar` as the root
/// widget.
#[macro_export]
macro_rules! scrollbar {
  ($($t: tt)*) => { fn_widget! { @Scrollbar { $($t)* } } };
}
pub use scrollbar;

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

pub struct ScrollbarDeclarer(FatObj<()>);

impl Declare for Scrollbar {
  type Builder = ScrollbarDeclarer;

  fn declarer() -> Self::Builder { ScrollbarDeclarer(FatObj::new(())) }
}

impl ObjDeclarer for ScrollbarDeclarer {
  type Target = FatObj<Scrollbar>;
  fn finish(mut self) -> Self::Target {
    let scroll = self
      .0
      .take_scrollable_widget()
      .unwrap_or_else(|| Stateful::new(ScrollableWidget::default()));
    self.0.map(|_| Scrollbar { scroll })
  }
}

impl<'c> ComposeChild<'c> for Scrollbar {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let scroll = this.read().scroll.clone_writer();
    // Here we provide the `ScrollableWidget`, which allows the theme to access
    // scroll states or enables descendants to trigger scrolling to a different
    // position.
    providers! {
      providers: [Provider::writer(scroll.clone_writer(), None)],
      @ {
        let h_scrollbar = distinct_pipe!($read(scroll).is_x_scrollable())
          .map(move |need_bar| need_bar.then(|| fn_widget!{
            let mut h_track = @Stack {
              class: H_SCROLL_TRACK,
              h_align: HAlign::Stretch,
              on_wheel: move |e| $write(scroll).scroll(-e.delta_x, -e.delta_y),
            };
            let mut h_thumb =  @Container {
              class: H_SCROLL_THUMB,
              size: distinct_pipe!{
                let track_width = *$read(h_track.layout_width());
                let width = h_thumb_rate(&$read(scroll)) * track_width;
                Size::new(width, 4.)
              }
            };

            @(h_track) {
              on_tap: move |e| if e.is_primary {
                let rate = e.position().x / *$read(h_track.layout_width());
                let mut scroll = $write(scroll);
                let x = rate * scroll.max_scrollable().x;
                let scroll_pos = Point::new(x, scroll.get_scroll_pos().y);
                scroll.jump_to(scroll_pos);
              },
              @(h_thumb) {
                anchor: distinct_pipe!{
                  let rate = $read(scroll).get_x_scroll_rate();
                  let track_width = *$read(h_track.layout_width());
                  let thumb_width = *$read(h_thumb.layout_width());
                  Anchor::left(rate * (track_width - thumb_width))
                }
              }
            }
          }));

        let v_scrollbar = distinct_pipe!($read(scroll).is_y_scrollable())
          .map(move |need_bar| need_bar.then(|| fn_widget!{
            let mut v_track = @Stack {
              class: V_SCROLL_TRACK,
              v_align: VAlign::Stretch,
              on_wheel: move |e| $write(scroll).scroll(-e.delta_x, -e.delta_y),
            };

            let mut v_thumb = @Container {
              class: V_SCROLL_THUMB,
              size: distinct_pipe!{
                let track_height = *$read(v_track.layout_height());
                let height = v_thumb_rate(&$read(scroll)) * track_height;
                Size::new(4., height)
              }
            };

            @(v_track) {
              on_tap: move |e| if e.is_primary {
                let rate = e.position().y / *$read(v_track.layout_height());
                let mut scroll = $write(scroll);
                let y = rate * scroll.max_scrollable().y;
                let scroll_pos = Point::new(scroll.get_scroll_pos().x, y);
                scroll.jump_to(scroll_pos);
              },
              @(v_thumb) {
                anchor: distinct_pipe!{
                  let rate = $read(scroll).get_y_scroll_rate();
                  let track_height = *$read(v_track.layout_height());
                  let thumb_height = *$read(v_thumb.layout_height());
                  Anchor::top(rate * (track_height - thumb_height))
                }
              }
            }
          }));

        let mut scroll = FatObj::new(scroll);
        @Stack {
          fit: StackFit::Passthrough,
          @(scroll) {
            class: SCROLL_CLIENT_AREA,
            @{ child }
          }
          @InParentLayout { @{ h_scrollbar } }
          @InParentLayout { @{ v_scrollbar } }
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

impl std::ops::Deref for ScrollbarDeclarer {
  type Target = FatObj<()>;
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for ScrollbarDeclarer {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
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
      @(scrollbar) {
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
        @(scrollbar) {
          @Container { size: Size::new(500., 500.) }
        }
      })
      .with_wnd_size(Size::new(100., 100.))
      .on_initd(move |wnd| {
        // Trigger a frame before scrolling to ensure the scrollbar is generated by the
        // pipe.
        AppCtx::new_test_frame(wnd);
        inner2.write().scroll(50., 50.);
        AppCtx::new_test_frame(wnd);
      })
    },
    LayoutCase::default().with_size(Size::new(100., 100.))
  );
}
