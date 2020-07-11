// use super::box_constraint::BoxBound;
use super::flex::FlexFit;
use crate::prelude::Size;
use crate::render::render_ctx::RenderCtx;
use crate::render::render_tree::*;
use crate::render::*;
use crate::widget::Widget;

// Expand box in the layout container will auto fill the area.
#[derive(Debug)]
struct ExpandBox {
  flex: i32,
  fit: FlexFit,
  child: Box<dyn Widget>,
}
#[derive(Debug)]
struct ExpandBoxRender {
  flex: i32,
  fit: FlexFit,
}

impl RenderWidget for ExpandBox {
  type RO = ExpandBoxRender;
  fn create_render_object(&self) -> Self::RO {
    ExpandBoxRender {
      flex: self.flex,
      fit: self.fit,
    }
  }
}

pub trait FlexElem {
  fn flex(&self) -> Option<i32>;
  fn fit(&self) -> Option<FlexFit>;
}

impl<O> FlexElem for O {
  default fn flex(&self) -> Option<i32> { None }
  default fn fit(&self) -> Option<FlexFit> { None }
}

impl FlexElem for ExpandBox {
  fn flex(&self) -> Option<i32> { Some(self.flex) }
  fn fit(&self) -> Option<FlexFit> { Some(self.fit) }
}

impl RenderObject for ExpandBoxRender {
  type Owner = ExpandBox;
  fn update(&mut self, owner: &ExpandBox) {
    self.fit = owner.fit;
    self.flex = owner.flex;
  }

  #[inline]
  fn perform_layout(&mut self, _id: RenderId, _ctx: &mut RenderCtx) -> Size { Size::zero() }

  #[inline]
  fn get_constraints(&self) -> LayoutConstraints { LayoutConstraints::EFFECTED_BY_PARENT }

  fn paint<'a>(&'a self, _ctx: &mut PaintingContext<'a>) {}
}
