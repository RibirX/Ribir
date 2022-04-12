#![feature(negative_impls)]
#![feature(core_intrinsics)]

use animation::animation_ctrl::new_animation_ctrl;
use animation::animation_ctrl::AnimationByTicker;
use animation::curve::ease_in_expo;
use animation::ticker_animation_mgr::new_ticker_animation_mgr;
use animation::tween::AnimationTween;
use ribir::animation::RepeatMode;

use ribir::prelude::*;
use std::intrinsics::ceilf32;
use std::time::Duration;

struct Demo {}

impl CombinationWidget for Demo {
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    let col = Column::default();

    let ins = Text {
      text: "click me to begin the animation".into(),
      style: <_>::default(),
    }
    .into_stateful();

    let target = Text {
      text: text::literal!("0"),
      style: <_>::default(),
    }
    .into_stateful();
    let bd = BoxDecoration {
      background: None,
      border: None,
      radius: None,
    }
    .into_stateful();
    let padding = Padding { padding: EdgeInsets::all(10.) }.into_stateful();
    let mut bd_state = unsafe { bd.state_ref().clone() };
    let mut padding_state = unsafe { padding.state_ref() };
    let mut target_state = unsafe { target.state_ref().clone() };

    let mut ticker1 = ctx
      .ticker_ctrl(Duration::from_secs(5))
      .unwrap()
      .with_round()
      .with_repeat(RepeatMode::Infinity);

    let mut ticker2 = ctx
      .ticker_ctrl(Duration::from_secs(5))
      .unwrap()
      .with_round()
      .with_repeat(RepeatMode::Repeat(5));

    let mut ctrl1 = new_animation_ctrl(Some(ease_in_expo()));
    let mut ctrl2 = new_animation_ctrl(None);

    ctrl1
      .tween(Color::new(0., 0.5, 0.2, 0.2), Color::new(1., 0.5, 0.2, 0.2))
      .subscribe(move |color| {
        bd_state.background = Some(Brush::Color(color));
      });
    ctrl1
      .tween(EdgeInsets::all(10.), EdgeInsets::all(0.))
      .subscribe(move |padding| {
        padding_state.padding = padding;
      });

    ctrl2
      .subject()
      .map(|p| {
        let src = "1234567890".to_string();
        let len = src.len();
        let new_len = unsafe { ceilf32(p * (src.len() as f32)) as usize } % (len + 1);
        let (text, _) = src.split_at(new_len);
        text.to_string()
      })
      .subscribe(move |text| {
        if *target_state.text != text {
          target_state.text = text.into();
        }
      });

    ctrl1.trigger_by(&mut *ticker1);
    ctrl2.trigger_by(&mut *ticker2);

    let w = ins.on_tap(move |_| {
      ticker1.restart(true);
      ticker2.restart(true);
    });

    let items = vec![
      w.box_it(),
      padding.have_child(bd.have_child(target)).box_it(),
    ];
    return col.have_child(items).box_it();
  }
}
fn main() {
  let demo = Demo {}.into_stateful();

  Application::new().run(demo.box_it(), Some(new_ticker_animation_mgr()));
}
