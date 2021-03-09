#[derive(Debug)]
pub enum CanvasError {
  /// atlas is too full to store the texture, buf the texture is good for store
  /// in the atlas if it's not store too many others.
  TextureSpaceNotEnough,
  /// The layer id is invalid, maybe its cache be cleaned.
  InValidLayerId,
}
