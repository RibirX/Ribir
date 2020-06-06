#[derive(Debug)]
pub enum CanvasError {
  /// atlas is too full to store the texture, buf the texture is good for store
  /// in the atlas if it's not store too many others.
  TextureSpaceNotEnough,
  /// The resource you want to store in the atlas is too large, you should not
  /// try to store it again. Maybe paint it in a single draw.
  ResourceOverTheMaxLimit,
  /// The layer id is invalid, maybe its cache be cleaned.
  InValidLayerID,
}
