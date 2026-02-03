use std::cell::Cell;

use crate::prelude::*;

/// Defines how a widget should be scaled and positioned within its container
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub enum BoxFit {
  /// Widget maintains original size without scaling
  #[default]
  None,
  /// Stretches widget to fill container, ignoring aspect ratio
  Fill,
  /// Scales widget uniformly (maintaining aspect ratio) to fit within
  /// container, potentially leaving empty spaces
  Contain,
  /// Scales widget uniformly to completely cover container,
  /// potentially clipping content
  Cover,
  /// Scales widget to fully cover container's height while maintaining aspect
  /// ratio, potentially clipping content
  CoverHeight,
  /// Scales widget to fully cover container's width while maintaining aspect
  /// ratio, potentially clipping content
  CoverWidth,
}

/// A widget that scales and positions its child according to a `BoxFit`
/// strategy.
///
/// `FittedBox` applies a scaling transform (and optional clipping) so the
/// child fits the available space as defined by the chosen strategy. It keeps
/// the computed scale factor for rendering and transform queries.
///
/// # Example
///
/// Scale the inner red container to cover the width of the outer container.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(200., 200.),
///   @Container {
///     box_fit: BoxFit::CoverWidth,
///     size: Size::new(50., 100.),
///     background: Color::RED,
///   }
/// };
/// ```
#[derive(SingleChild, Default)]
pub struct FittedBox {
  /// The fitting strategy to apply
  pub box_fit: BoxFit,
  /// Stores calculated scale factors for rendering transformation
  scale_factor: Cell<Vector>,
}

impl Declare for FittedBox {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl FittedBox {
  /// Creates a FittedBox with specified scaling strategy
  pub fn new(box_fit: BoxFit) -> Self { Self { box_fit, scale_factor: Cell::default() } }

  fn measure_child(
    &self, clamp: BoxClamp, ctx: &mut MeasureCtx, max_valid: impl FnOnce(Size) -> bool,
    min_valid: impl Fn(Size) -> bool, fit_scale: impl FnOnce(Size, Size) -> Vector,
  ) -> Size {
    let mut container = if max_valid(clamp.max) {
      clamp.max
    } else if min_valid(clamp.min) {
      clamp.min
    } else {
      self.scale_factor.set(Vector::one());
      return ctx.layout_child(ctx.assert_single_child(), clamp);
    };

    let child = ctx.assert_single_child();
    let child_size = ctx.layout_child(child, BoxClamp::default());

    if !min_valid(child_size) {
      self.scale_factor.set(Vector::one());
      return clamp.clamp(child_size);
    }

    let scale = fit_scale(child_size, container);
    self.scale_factor.set(scale);

    if !container.width.is_finite() {
      container.width = child_size.width * scale.x;
    }
    if !container.height.is_finite() {
      container.height = child_size.height * scale.y;
    }

    clamp.clamp(container)
  }
}

impl Render for FittedBox {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    match self.box_fit {
      BoxFit::None => {
        self.scale_factor.set(Vector::one());
        ctx.layout_child(ctx.assert_single_child(), clamp)
      }
      BoxFit::Fill => self.measure_child(
        clamp,
        ctx,
        Size::is_finite,
        |size| !size.is_empty(),
        |child, container| {
          Vector::new(container.width / child.width, container.height / child.height)
        },
      ),
      BoxFit::Contain => self.measure_child(
        clamp,
        ctx,
        |size| size.width.is_finite() || size.height.is_finite(),
        |size| !size.is_empty(),
        |child, container| {
          let scale = f32::min(container.width / child.width, container.height / child.height);
          Vector::splat(scale)
        },
      ),
      BoxFit::Cover => self.measure_child(
        clamp,
        ctx,
        Size::is_finite,
        |size| !size.is_empty(),
        |child, container| {
          let scale = f32::max(container.width / child.width, container.height / child.height);
          Vector::splat(scale)
        },
      ),
      BoxFit::CoverWidth => self.measure_child(
        clamp,
        ctx,
        |size| size.width.is_finite(),
        |size| size.width > 0.,
        |child, container| {
          let scale = container.width / child.width;
          Vector::splat(scale)
        },
      ),
      BoxFit::CoverHeight => self.measure_child(
        clamp,
        ctx,
        |size| size.height.is_finite(),
        |size| size.height > 0.,
        |child, container| {
          let scale = container.height / child.height;
          Vector::splat(scale)
        },
      ),
    }
  }

  fn place_children(&self, size: Size, ctx: &mut PlaceCtx) {
    let child = ctx.assert_single_child();
    let child_size = ctx
      .tree
      .store
      .layout_info(child)
      .map(|i| i.size.unwrap())
      .unwrap_or_default();
    let scale = self.scale_factor.get();
    let mut pos = Point::zero();

    pos.x = center_align(size.width, child_size.width, scale.x);
    pos.y = center_align(size.height, child_size.height, scale.y);

    ctx.update_position(child, pos);
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    let clip_rect = Rect::from_size(ctx.box_size()?);
    ctx.clip(clip_rect);
    Some(clip_rect)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let scale = self.scale_factor.get();
    if matches!(self.box_fit, BoxFit::Cover | BoxFit::CoverHeight | BoxFit::CoverWidth) {
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

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("fittedBox") }

  fn get_transform(&self) -> Option<Transform> {
    let scale = self.scale_factor.get();
    Some(Transform::scale(scale.x, scale.y))
  }
}

fn center_align(container: f32, child: f32, scale: f32) -> f32 { (container / scale - child) / 2.0 }

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

      let (fit, w_fit) = split_value(FittedBox { box_fit, scale_factor: <_>::default() });

      let w = fn_widget! {
        let w_fit = w_fit.clone_writer();
        @(w_fit) { @MockBox { size } }
      };
      let wnd = TestWindow::new_with_size(w, WND_SIZE);
      wnd.draw_frame();
      wnd.assert_root_size(expect);
      assert_eq!(fit.read().scale_factor.get(), expected_scale);
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
      expect: WND_SIZE,
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

  widget_layout_test!(
    contain_in_the_center,
    WidgetTester::new(mock_box! {
      size: Size::splat(100.),
      @MockBox {
        size: Size::new(100., 200.),
        box_fit: BoxFit::Contain,
      }
    }),
    LayoutCase::new(&[0, 0, 0])
      .with_size(Size::new(100., 200.))
      .with_pos(Point::new(50., 0.)),
  );
}
