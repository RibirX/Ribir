#[derive(Debug)]
pub enum Error {
  /// atlas is full unable to store the texture and the texture is grow to its
  /// limit, but the texture is good for store in the atlas if it's not store
  /// too many others.
  TextureSpaceLimit,
  /// The image is too large to good for the atlas store.
  LargeImageAvoid,
}
