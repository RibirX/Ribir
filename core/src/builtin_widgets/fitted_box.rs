use std::cell::Cell;

use crate::prelude::*;

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub enum BoxFit {
  /// Widget will not be scale.
  #[default]
  None,
  /// The entire widget will completely fill its container. If the widget's
  /// aspect ratio does not match the aspect ratio of its box, then the widget
  /// will be stretched to fit.
  Fill,
  /// Widget is scaled to maintain its aspect ratio while fitting within the
  /// container box. The entire widget is made to fill the box, while preserving
  /// its aspect ratio,
  Contain,
  /// Widget is scale to maintain its aspect ratio while filling to full cover
  /// its container box. If the widget's aspect ratio does not the aspect ratio
  /// of its box, then the widget will be clipped to fit.
  Cover,

  /// The widget scales to maintain its aspect ratio while filling the full
  /// coverage Y direction of its container box.
  CoverY,

  /// The widget scales to maintain its aspect ratio while filling the full
  /// coverage X direction of its container box.
  CoverX,
}

/// Widget set how its child should be scale to fit its box.
#[derive(SingleChild, Default)]
pub struct FittedBox {
  pub box_fit: BoxFit,
  scale_cache: Cell<Vector>,
}

impl Declare for FittedBox {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl FittedBox {
  pub fn new(box_fit: BoxFit) -> Self { Self { box_fit, scale_cache: <_>::default() } }
}

impl Render for FittedBox {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let container = clamp.max;
    if container.is_empty() {
      self.scale_cache.set(Vector::zero());
      return Size::zero();
    }

    let child_size =
      ctx.assert_perform_single_child_layout(BoxClamp { min: clamp.min, max: INFINITY_SIZE });

    if child_size.is_empty() {
      self.scale_cache.set(Vector::zero());
      return child_size;
    }

    let x = if container.width.is_finite() { container.width / child_size.width } else { 1. };
    let y = if container.height.is_finite() { container.height / child_size.height } else { 1. };
    let scale = match self.box_fit {
      BoxFit::None => Vector::new(1., 1.),
      BoxFit::Fill => Vector::new(x, y),
      BoxFit::Contain => {
        let scale = x.min(y);
        Vector::new(scale, scale)
      }
      BoxFit::Cover => {
        let scale = x.max(y);
        Vector::new(scale, scale)
      }
      BoxFit::CoverY => Vector::new(y, y),
      BoxFit::CoverX => Vector::new(x, x),
    };
    self.scale_cache.set(scale);
    let size = Size::new(child_size.width * scale.x, child_size.height * scale.y);
    clamp.clamp(size)
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    let clip_rect = Rect::from_size(ctx.box_size()?);
    ctx.clip(clip_rect);
    Some(clip_rect)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let scale = self.scale_cache.get();
    if matches!(self.box_fit, BoxFit::Cover) {
      let size = ctx.box_size().unwrap();
      let child_size = ctx
        .single_child_box()
        .expect("Should always have a single child")
        .size;
      if size.width < child_size.width * scale.x || size.height < child_size.height * scale.y {
        let path = Path::rect(&Rect::from(size));
        ctx.painter().clip(path.into());
      }
    }

    ctx.painter().scale(scale.x, scale.y);
  }

  fn get_transform(&self) -> Option<Transform> {
    let scale = self.scale_cache.get();
    Some(Transform::scale(scale.x, scale.y))
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  const WND_SIZE: Size = Size::new(300., 300.);

  struct FitTestCase {
    box_fit: BoxFit,
    size: Size,
    expect: Size,
    expected_scale: Vector,
  }

  impl FitTestCase {
    fn test(self) {
      let Self { box_fit, size, expect, expected_scale } = self;

      let (fit, w_fit) = split_value(FittedBox { box_fit, scale_cache: <_>::default() });

      let w = fn_widget! {
        let w_fit = w_fit.clone_writer();
        @$w_fit { @MockBox { size } }
      };
      let mut wnd = TestWindow::new_with_size(w, WND_SIZE);
      wnd.draw_frame();
      wnd.assert_root_size(expect);
      assert_eq!(fit.read().scale_cache.get(), expected_scale);
    }
  }

  #[test]
  fn fit_test() {
    reset_test_env!();

    let small_size: Size = Size::new(100., 150.);

    FitTestCase {
      box_fit: BoxFit::None,
      size: small_size,
      expect: small_size,
      expected_scale: Vector::new(1., 1.),
    }
    .test();

    FitTestCase {
      box_fit: BoxFit::Fill,
      size: small_size,
      expect: WND_SIZE,
      expected_scale: Vector::new(3., 2.),
    }
    .test();

    FitTestCase {
      box_fit: BoxFit::Cover,
      size: small_size,
      expect: WND_SIZE,
      expected_scale: Vector::new(3., 3.),
    }
    .test();

    let big_size_clip = Size::new(600., 900.);
    FitTestCase {
      box_fit: BoxFit::Cover,
      size: big_size_clip,
      expect: WND_SIZE,
      expected_scale: Vector::new(0.5, 0.5),
    }
    .test();

    FitTestCase {
      box_fit: BoxFit::Contain,
      size: small_size,
      expect: Size::new(200., 300.),
      expected_scale: Vector::new(2., 2.),
    }
    .test();
  }

  widget_layout_test!(
    as_builtin_field,
    WidgetTester::new(fn_widget! {
      @MockBox {
        size: Size::new(200., 200.),
        box_fit: BoxFit::Fill,
      }
    })
    .with_wnd_size(WND_SIZE),
    LayoutCase::default().with_size(WND_SIZE)
  );
}
