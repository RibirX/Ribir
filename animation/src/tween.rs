use ribir::animation::AnimationCtrl;
use ribir::widget::Observable;
use rxrust::ops::box_it::LocalBoxOp;

use crate::with_tween::WithTween;

pub trait AnimationTween<T> {
  fn tween(&mut self, begin: T, end: T) -> LocalBoxOp<'static, T, ()>;
}

impl<T> AnimationTween<T> for dyn AnimationCtrl
where
  T: 'static + WithTween,
{
  #[inline]
  fn tween(&mut self, begin: T, end: T) -> LocalBoxOp<'static, T, ()> {
    let sub = self.subject();
    sub.map(move |t| WithTween::tween(&begin, &end, t)).box_it()
  }
}
