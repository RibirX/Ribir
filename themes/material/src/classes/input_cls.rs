use ribir_core::prelude::*;
use ribir_widgets::input::{INPUT, TEXT_CARET, TEXT_SELECTION, TEXTAREA};

use crate::md;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(TEXT_CARET, |w| {
    rdl! {
      let mut w = FatObj::new(w);
      let blink_interval = Duration::from_millis(500);
      let u = Local::interval(blink_interval)
        .subscribe(move |idx| *$write(w.opacity()) = (idx % 2) as f32);
      let border = BuildCtx::color()
        .map(|color| Border::only_left(BorderSide::new(2., (*color).into())));
      @(w) {
        clamp: BoxClamp::fixed_width(2.),
        border,
        on_disposed: move |_| u.unsubscribe()
      }
    }
    .into_widget()
  });

  classes.insert(
    TEXT_SELECTION,
    style_class! {
      background: {
        let color = BuildCtx::color();
        color.into_container_color(BuildCtx::get()).map(|c| c.with_alpha(0.8))
      }
    },
  );

  fn input_border(w: Widget) -> Widget {
    let mut w = FatObj::new(w);
    let blur = Palette::of(BuildCtx::get()).on_surface_variant();

    let focus_watcher = w.is_focused();
    let border = BuildCtx::color().map_with_watcher(focus_watcher, move |c, focus| {
      let color = if *focus { *c } else { blur };
      Border::all(BorderSide::new(1., color.into()))
    });

    w.with_border(border).with_radius(md::RADIUS_2);
    w.into_widget()
  }
  classes.insert(INPUT, input_border);
  classes.insert(TEXTAREA, input_border);
}
