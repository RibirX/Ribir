use ribir::animation::AnimationCtrl;
use ribir::prelude::Tween;
use ribir::widget::Observable;
use rxrust::ops::box_it::LocalBoxOp;

pub trait AnimationTween<T> {
  fn tween(&mut self, begin: T, end: T) -> LocalBoxOp<'static, T, ()>;
}

impl<T> AnimationTween<T> for dyn AnimationCtrl
where
  T: 'static + Tween,
{
  #[inline]
  fn tween(&mut self, begin: T, end: T) -> LocalBoxOp<'static, T, ()> {
    let sub = self.subject();
    sub.map(move |t| Tween::tween(&begin, &end, t)).box_it()
  }
}
