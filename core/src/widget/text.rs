use crate::prelude::*;
use crate::render::font::*;
use crate::render::render_ctx::*;
use crate::render::render_tree::*;
use crate::render::*;
pub use canvas::{Color, FillStyle};
use std::sync::Arc;
#[derive(Debug, Default)]
pub struct SingleTextRender {
  pub text: String,
  pub style: Option<Arc<FontStyle>>,
  pub compose: Option<Arc<FontStyle>>,
}

macro cover_field($tar: tt, $val: tt, $field: tt) {
  if $tar.$field.is_none() {
    $tar.$field = $val.$field.clone();
  }
}

pub fn compose_text_style(
  base: Option<Arc<FontStyle>>,
  cover: Option<Arc<FontStyle>>,
) -> Option<Arc<FontStyle>> {
  if base.is_none() {
    return cover.clone();
  }

  if cover.is_none() {
    return base.clone();
  }
  let mut res = base.unwrap().clone();
  let mut style = Arc::make_mut(&mut res);
  let content = cover.unwrap();
  cover_field!(style, content, font_color);
  cover_field!(style, content, font_size);
  cover_field!(style, content, font_weight);
  cover_field!(style, content, bold);
  cover_field!(style, content, italics);

  return Some(res);
}

impl SingleTextRender {
  #[inline]
  fn reset(&mut self, owner: &Text) {
    let text = if owner.text.is_some() {
      owner.text.clone().unwrap()
    } else {
      "".to_string()
    };
    self.text = text;
    self.style = owner.style.as_ref().map(|style| Arc::new(style.clone()));
  }

  fn compose_style(&mut self, inherit: Option<Arc<FontStyle>>) {
    self.compose = compose_text_style(self.style.clone(), inherit);
  }

  fn place(&mut self, ctx: &mut RenderCtx, self_size: Size, sizes: &mut Vec<Size>) -> Size {
    let mut s = self_size;
    for &size in sizes.iter() {
      s.width += size.width;
      s.height = s.height.max(size.height);
    }

    let mut pos = self_size.width;
    let mut child_iter = ctx.children();
    let mut idx = 0;
    while let Some(mut child_ctx) = child_iter.next() {
      let offset_y = s.height - sizes[idx].height;
      child_ctx.update_position(Point::new(pos, offset_y));
      pos += sizes[idx].width;
      idx += 1;
    }

    return s;
  }
}

impl RenderObject for SingleTextRender {
  type Owner = Text;
  #[inline]
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    if self.compose.is_none() {
      self.compose_style(None);
    }
    let mut sizes = vec![];
    let self_size = ctx.mesure_text(&self.text, &self.compose).size;

    {
      let mut child_iter = ctx.children();
      while let Some(mut child_ctx) = child_iter.next() {
        child_ctx
          .render_obj_mut()
          .downcast_mut::<SingleTextRender>()
          .map(|text_render| text_render.compose_style(self.compose.clone()));

        sizes.push(child_ctx.perform_layout(clamp.clone()));
        print!("{}", sizes.len());
      }
    }

    self.place(ctx, self_size, &mut sizes)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn update<'a>(&mut self, owner_widget: &Text, ctx: &mut UpdateCtx) {
    self.reset(owner_widget);
    ctx.mark_needs_layout();
  }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    let painter = ctx.painter();
    painter.set_font(to_font(&self.compose));
    painter.set_style(font_color(&self.compose));
    painter.fill_text(&self.text, None);
  }
}

#[derive(Debug)]
pub struct Text {
  pub text: Option<String>,
  pub children: Option<SmallVec<[BoxWidget; 1]>>,
  pub style: Option<FontStyle>,
}

render_widget_base_impl!(Text);

impl RenderWidget for Text {
  type RO = SingleTextRender;
  fn create_render_object(&self) -> Self::RO {
    let mut ro = SingleTextRender::default();
    ro.reset(&self);
    ro
  }

  #[inline]
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> {
    self
      .children
      .as_mut()
      .map(|children| std::mem::replace(children, smallvec![]))
  }
}
