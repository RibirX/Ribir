use algo::FrameCache;
use painter::ShallowImage;

pub struct TextureRecords {
  id_from: usize,
  next_id: usize,
  cache: FrameCache<ShallowImage, usize>,
}

impl TextureRecords {
  pub fn new(id_from: usize) -> Self {
    Self {
      id_from,
      next_id: id_from,
      cache: <_>::default(),
    }
  }
  pub fn get_id(&mut self, image: &ShallowImage) -> Option<usize> { self.cache.get(image).cloned() }

  pub fn insert(&mut self, image: ShallowImage) -> usize {
    *self.cache.get_or_insert_with_key(image, |_| {
      let (id, _) = self.next_id.overflowing_add(1);
      self.next_id = id.max(self.id_from);
      self.next_id
    })
  }

  pub fn frame_end(&mut self) { self.cache.frame_end("Texture"); }
}
