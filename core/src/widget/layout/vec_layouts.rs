use crate::prelude::*;
#[derive(Clone, Debug)]
struct LayoutInfo {
  size: Option<Size>,
  pos: Option<Point>,
}

#[derive(Debug, Default)]
pub struct VecLayouts(Vec<Option<LayoutInfo>>);

impl VecLayouts {
  pub fn new() -> VecLayouts { <_>::default() }
  pub fn reset(&mut self, cnt: usize) {
    self.0.clear();
    self.0.resize(cnt, None);
  }

  pub fn update_size(&mut self, idx: usize, size: Size) {
    if idx >= self.0.len() {
      return;
    }
    let val = &mut self.0[idx].get_or_insert(LayoutInfo {
      size: None,
      pos: None,
    });

    val.size = Some(size);
  }

  pub fn update_position(&mut self, idx: usize, pos: Point) {
    if idx >= self.0.len() {
      return;
    }
    let val = &mut self.0[idx].get_or_insert(LayoutInfo {
      size: None,
      pos: None,
    });
    val.pos = Some(pos);
  }

  pub fn size(&self, idx: usize) -> Option<Size> {
    if idx >= self.0.len() {
      return None;
    }
    self.0[idx].as_ref().and_then(|val| val.size)
  }

  pub fn position(&self, idx: usize) -> Option<Point> {
    if idx >= self.0.len() {
      return None;
    }
    self.0[idx].as_ref().and_then(|val| val.pos)
  }
}
