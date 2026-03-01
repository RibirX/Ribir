//! WebP image with lazy per-frame streaming decode and caching.
//!
//! This module only supports WebP format. Use the `asset!` macro to
//! automatically convert other formats (PNG, JPEG, GIF, etc.) to WebP
//! at compile time.
//!
//! Frame decoding is lazy - pixels are only decoded when first accessed.
//! For animated images, frames are decoded sequentially up to the requested
//! frame.
//!
//! # Usage as Widget
//!
//! `Image` implements `Compose`, so it can be used directly as a widget:
//!
//! ```rust ignore
//! use ribir::prelude::*;
//!
//! fn_widget! {
//!   // asset! converts PNG/JPEG/GIF to WebP at compile time
//!   let img: Image = asset!("./image.png", "image");
//!   @{ img }  // Static image renders first frame
//! }
//! ```
//!
//! For animated images, playback starts automatically and loops according
//! to the image's loop count setting.
//!
//! # Shared Decoding
//!
//! Cloning an `Image` is cheap (reference-counted). All clones share the same
//! decoded frame cache, avoiding redundant decoding when the same image is
//! used in multiple places:
//!
//! ```rust ignore
//! let img: Image = asset!("logo.png", "image");
//! // All three share the same decoded frame cache
//! @Row {
//!   @ { img.clone() }
//!   @ { img.clone() }
//!   @ { img }
//! }
//! ```
//!
//! # Custom Animation Control
//!
//! For custom animation control (pause, seek, manual frame stepping), use
//! the frame access APIs instead of the default `Compose` implementation:
//!
//! - [`Image::frame(index)`] - Get a specific frame
//! - [`Image::frame_iter()`] - Iterate over all frames
//! - [`DecodedFrame::delay_ms`] - Frame display duration
//!
//! ```rust ignore
//! // Manual frame control example
//! fn_widget! {
//!   let img: Image = asset!("./animation.gif", "image");
//!   let frame_idx = Stateful::new(0usize);
//!   
//!   // Render specific frame manually
//!   @{ img.frame(*$read(frame_idx)).map(|f| f.image) }
//! }
//! ```

use std::{
  borrow::Cow,
  io::{BufRead, Read, Seek, SeekFrom},
  pin::Pin,
  sync::{Arc, Mutex, OnceLock},
};

use image_webp::{DecodingError, WebPDecoder};
use ribir_algo::Resource;
use ribir_geom::DeviceSize;
use ribir_painter::{ColorFormat, PixelImage};

use crate::prelude::*;

// ============================================================================
// Public Types
// ============================================================================

/// Loop count for animated images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopCount {
  #[default]
  Infinite,
  Finite(u32),
}

/// A decoded frame with image data and display delay.
#[derive(Clone)]
pub struct DecodedFrame {
  pub image: Resource<PixelImage>,
  /// Frame delay in milliseconds (0 for static images).
  pub delay_ms: u32,
}

/// WebP image with lazy streaming decode and frame caching.
///
/// Source images (PNG, JPEG, GIF, etc.) are converted to WebP at compile time.
/// At runtime, frames are decoded on-demand and cached for reuse.
///
/// Cloning an `Image` is cheap (reference-counted). All clones share the same
/// decoded frame cache, avoiding redundant decoding when the same image is
/// used in multiple places.
#[derive(Clone)]
pub struct Image(Arc<ImageInner>);

/// Iterator over image frames.
pub struct FrameIterator<'a> {
  image: &'a Image,
  index: usize,
}

// ============================================================================
// Image Implementation
// ============================================================================

impl Image {
  /// Creates an Image from raw WebP data.
  ///
  /// Only parses the WebP header for metadata. No frame decoding until access.
  pub fn new(raw: impl Into<Cow<'static, [u8]>>) -> Result<Self, DecodingError> {
    let raw = StableData::from_cow(raw.into());
    let decoder = WebPDecoder::new(std::io::Cursor::new(raw.as_slice()))?;

    let (width, height) = decoder.dimensions();
    let is_animated = decoder.is_animated();
    let frame_count = if is_animated { decoder.num_frames() as usize } else { 1 };
    let loop_count = match decoder.loop_count() {
      image_webp::LoopCount::Forever => LoopCount::Infinite,
      image_webp::LoopCount::Times(n) => LoopCount::Finite(n.get() as u32),
    };

    Ok(Self(Arc::new(ImageInner {
      decoder_state: Mutex::new(DecoderState::new()),
      raw,
      width,
      height,
      loop_count,
      is_animated,
      frame_cache: new_frame_cache(frame_count),
    })))
  }

  /// Creates an Image from raw WebP data and pre-decoded frames.
  ///
  /// Used for deserialization or compile-time decoded assets.
  pub fn from_parts(
    raw: impl Into<Cow<'static, [u8]>>, width: u32, height: u32, loop_count: LoopCount,
    frames: Vec<DecodedFrame>,
  ) -> Self {
    let frame_count = frames.len();
    let frame_cache: Box<[_]> = frames
      .into_iter()
      .map(|f| {
        let lock = OnceLock::new();
        let _ = lock.set(f);
        lock
      })
      .collect();

    Self(Arc::new(ImageInner {
      decoder_state: Mutex::new(DecoderState::with_decoded(frame_count)),
      raw: StableData::from_cow(raw.into()),
      width,
      height,
      loop_count,
      is_animated: frame_count > 1,
      frame_cache,
    }))
  }

  // --- Metadata ---

  /// Returns the image dimensions as a `DeviceSize`.
  #[inline]
  pub fn size(&self) -> DeviceSize { DeviceSize::new(self.0.width as i32, self.0.height as i32) }

  /// Returns the image width in pixels.
  #[inline]
  pub fn width(&self) -> u32 { self.0.width }

  /// Returns the image height in pixels.
  #[inline]
  pub fn height(&self) -> u32 { self.0.height }

  /// Returns the number of frames in the image.
  #[inline]
  pub fn frame_count(&self) -> u32 { self.0.frame_cache.len() as u32 }

  /// Returns whether this is an animated image.
  #[inline]
  pub fn is_animated(&self) -> bool { self.0.is_animated }

  /// Returns the loop count for animated images.
  #[inline]
  pub fn loop_count(&self) -> LoopCount { self.0.loop_count }

  /// Returns total duration of all frames in milliseconds.
  ///
  /// Note: Requires decoding all frames to get their delays.
  pub fn total_duration_ms(&self) -> u64 {
    if !self.0.frame_cache.is_empty() {
      self.ensure_decoded_up_to(self.0.frame_cache.len() - 1);
    }
    self
      .0
      .frame_cache
      .iter()
      .filter_map(|l| l.get())
      .map(|f| f.delay_ms as u64)
      .sum()
  }

  // --- Frame Access ---

  /// Returns the decoded frame at index, or None if out of bounds.
  ///
  /// Frames are decoded on first access. For animated images, all frames
  /// up to the requested index are decoded (WebP sequential dependency).
  pub fn frame(&self, index: usize) -> Option<DecodedFrame> {
    if index >= self.0.frame_cache.len() {
      return None;
    }

    // Fast path: already decoded
    if let Some(frame) = self.0.frame_cache[index].get() {
      return Some(frame.clone());
    }

    // Slow path: decode
    self.ensure_decoded_up_to(index);
    self.0.frame_cache[index].get().cloned()
  }

  /// Returns the first frame. Panics if image has no frames.
  #[inline]
  pub fn first_frame(&self) -> DecodedFrame { self.frame(0).expect("Image has no frames") }

  /// Returns an iterator over all frames.
  #[inline]
  pub fn frame_iter(&self) -> FrameIterator<'_> { FrameIterator { image: self, index: 0 } }

  /// Returns the total number of frames considering loop count.
  ///
  /// For infinite loops, returns `None`. For finite loops, returns
  /// `frame_count * loop_times`.
  #[inline]
  pub fn global_frame_count(&self) -> Option<usize> {
    match self.0.loop_count {
      LoopCount::Infinite => None,
      LoopCount::Finite(n) => Some(self.0.frame_cache.len() * n as usize),
    }
  }

  /// Returns the frame at a global index (handles wrapping for loops).
  #[inline]
  pub fn frame_by_global_idx(&self, global: usize) -> Option<DecodedFrame> {
    self.frame(global % self.0.frame_cache.len())
  }

  // --- Internal ---

  fn ensure_decoded_up_to(&self, target: usize) {
    let inner = &self.0;
    let mut state = inner.decoder_state.lock().unwrap();
    if state.decoded_count > target {
      return;
    }

    let start = state.decoded_count;
    let decoder = state
      .decoder
      .get_or_insert_with(|| WebPDecoder::new(RawPtrReader::new(inner.raw.as_ptr())).unwrap());

    let buf_size = decoder.output_buffer_size().unwrap_or(0);
    let mut buf = vec![0u8; buf_size];

    if inner.is_animated {
      for i in start..=target {
        let delay_ms = decoder
          .read_frame(&mut buf)
          .expect("Failed to decode frame");
        let _ = inner.frame_cache[i].set(create_frame(inner, buf.clone(), delay_ms));
      }
    } else {
      assert_eq!(target, 0, "Static image has only one frame");
      decoder
        .read_image(&mut buf)
        .expect("Failed to decode image");
      let _ = inner.frame_cache[0].set(create_frame(inner, buf, 0));
    }

    state.decoded_count = target + 1;
  }
}

impl std::fmt::Debug for Image {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let decoded = self
      .0
      .decoder_state
      .lock()
      .map(|s| s.decoded_count)
      .unwrap_or(0);
    f.debug_struct("Image")
      .field("size", &format!("{}x{}", self.0.width, self.0.height))
      .field("frames", &format!("{}/{}", decoded, self.0.frame_cache.len()))
      .field("animated", &self.0.is_animated)
      .field("loop_count", &self.0.loop_count)
      .finish()
  }
}

// ============================================================================
// FrameIterator Implementation
// ============================================================================

impl Iterator for FrameIterator<'_> {
  type Item = DecodedFrame;

  fn next(&mut self) -> Option<Self::Item> {
    let frame = self.image.frame(self.index)?;
    self.index += 1;
    Some(frame)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let remaining = self.image.frame_count() as usize - self.index;
    (remaining, Some(remaining))
  }
}

impl ExactSizeIterator for FrameIterator<'_> {}

// ============================================================================
// Widget Implementations
// ============================================================================

impl Compose for Image {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    pipe! {
      if $read(this).is_animated() {
        let frame_idx = Stateful::new(0usize);
        pipe! {
          let img = $read(this);
          let idx = *$read(frame_idx);
          let frame = img.frame_by_global_idx(idx).expect("Invalid frame index");
          if img.global_frame_count().is_none_or(|c| idx + 1 < c) {
            Local::timer(Duration::from_millis(frame.delay_ms as u64))
              .subscribe(move |_| *$write(frame_idx) += 1);
          }
          frame.image
        }.into_widget()
      } else {
        $read(this).first_frame().image.into_widget()
      }
    }
    .into_widget()
  }
}

impl Render for Resource<PixelImage> {
  fn measure(&self, clamp: BoxClamp, _: &mut MeasureCtx) -> Size {
    let size = Size::new(self.width() as f32, self.height() as f32);
    clamp.clamp(size)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();
    let box_rect = Rect::from_size(size);
    let img_rect = Rect::from_size(Size::new(self.width() as f32, self.height() as f32));
    let painter = ctx.painter();
    if let Some(rc) = img_rect.intersection(&box_rect) {
      painter.draw_img(self.clone(), &rc, &Some(rc));
    }
  }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("image") }
}

// ============================================================================
// Internal Types
// ============================================================================

/// Internal shared image data.
struct ImageInner {
  // IMPORTANT: decoder_state MUST be declared before raw.
  // Rust drops fields in declaration order, ensuring the decoder (which holds
  // a pointer to raw) is dropped before raw.
  decoder_state: Mutex<DecoderState>,
  raw: StableData,
  width: u32,
  height: u32,
  loop_count: LoopCount,
  is_animated: bool,
  frame_cache: Box<[OnceLock<DecodedFrame>]>,
}

/// Raw image data with stable memory address for decoder pointer safety.
enum StableData {
  Static(&'static [u8]),
  Owned(Pin<Box<[u8]>>),
}

impl StableData {
  fn from_cow(cow: Cow<'static, [u8]>) -> Self {
    match cow {
      Cow::Borrowed(b) => Self::Static(b),
      Cow::Owned(v) => Self::Owned(Pin::new(v.into_boxed_slice())),
    }
  }

  fn as_ptr(&self) -> *const [u8] {
    match self {
      Self::Static(s) => *s,
      Self::Owned(b) => &**b,
    }
  }

  fn as_slice(&self) -> &[u8] {
    match self {
      Self::Static(s) => s,
      Self::Owned(b) => b,
    }
  }
}

/// Decoder state for streaming decode.
struct DecoderState {
  decoder: Option<WebPDecoder<RawPtrReader>>,
  decoded_count: usize,
}

impl DecoderState {
  fn new() -> Self { Self { decoder: None, decoded_count: 0 } }

  fn with_decoded(count: usize) -> Self { Self { decoder: None, decoded_count: count } }
}

/// Reader holding a raw pointer to pinned data.
///
/// # Safety
/// Safe as long as the pointed data outlives this reader and remains pinned.
struct RawPtrReader {
  data: *const [u8],
  pos: usize,
}

// SAFETY: Only reads from data, never writes. Data is protected by Mutex.
unsafe impl Send for RawPtrReader {}

impl RawPtrReader {
  fn new(data: *const [u8]) -> Self { Self { data, pos: 0 } }

  /// SAFETY: Caller must ensure pointer is valid.
  unsafe fn data(&self) -> &[u8] { unsafe { &*self.data } }
}

impl Read for RawPtrReader {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    let data = unsafe { self.data() };
    let remaining = data.len().saturating_sub(self.pos);
    let len = buf.len().min(remaining);
    if len > 0 {
      buf[..len].copy_from_slice(&data[self.pos..self.pos + len]);
      self.pos += len;
    }
    Ok(len)
  }
}

impl Seek for RawPtrReader {
  fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
    let len = unsafe { self.data() }.len() as i64;
    let new_pos = match pos {
      SeekFrom::Start(n) => n as i64,
      SeekFrom::End(n) => len + n,
      SeekFrom::Current(n) => self.pos as i64 + n,
    };

    if new_pos < 0 {
      return Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "seek to negative position",
      ));
    }

    self.pos = new_pos as usize;
    Ok(self.pos as u64)
  }
}

impl BufRead for RawPtrReader {
  fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
    let data = unsafe { self.data() };
    Ok(data.get(self.pos..).unwrap_or_default())
  }

  fn consume(&mut self, amt: usize) { self.pos += amt; }
}

fn new_frame_cache(count: usize) -> Box<[OnceLock<DecodedFrame>]> {
  let mut v = Vec::with_capacity(count);
  v.resize_with(count, OnceLock::new);
  v.into_boxed_slice()
}

fn create_frame(inner: &ImageInner, buf: Vec<u8>, delay_ms: u32) -> DecodedFrame {
  DecodedFrame {
    image: Resource::new(PixelImage::new(
      buf.into(),
      inner.width,
      inner.height,
      ColorFormat::Rgba8,
    )),
    delay_ms,
  }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn loop_count_default() {
    assert_eq!(LoopCount::default(), LoopCount::Infinite);
  }

  #[test]
  fn loop_count_equality() {
    assert_eq!(LoopCount::Finite(3), LoopCount::Finite(3));
    assert_ne!(LoopCount::Finite(3), LoopCount::Finite(5));
    assert_ne!(LoopCount::Infinite, LoopCount::Finite(1));
  }

  #[test]
  fn image_clone_shares_cache() {
    // Create pre-decoded frame
    let pixel_data = vec![255u8; 4]; // 1x1 RGBA
    let frame = DecodedFrame {
      image: Resource::new(PixelImage::new(pixel_data.into(), 1, 1, ColorFormat::Rgba8)),
      delay_ms: 0,
    };
    let img = Image::from_parts(Vec::new(), 1, 1, LoopCount::Infinite, vec![frame]);

    // Clone should share the same Arc
    let img2 = img.clone();

    // Both should point to the same inner data
    assert!(Arc::ptr_eq(&img.0, &img2.0));
  }

  #[test]
  fn from_parts_static_image() {
    let pixel_data = vec![255u8; 16]; // 2x2 RGBA
    let frame = DecodedFrame {
      image: Resource::new(PixelImage::new(pixel_data.into(), 2, 2, ColorFormat::Rgba8)),
      delay_ms: 0,
    };
    let img = Image::from_parts(Vec::new(), 2, 2, LoopCount::Infinite, vec![frame]);

    assert_eq!(img.width(), 2);
    assert_eq!(img.height(), 2);
    assert_eq!(img.frame_count(), 1);
    assert!(!img.is_animated());
    assert_eq!(img.loop_count(), LoopCount::Infinite);
  }

  #[test]
  fn from_parts_animated_image() {
    let frame1 = DecodedFrame {
      image: Resource::new(PixelImage::new(vec![255u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
      delay_ms: 100,
    };
    let frame2 = DecodedFrame {
      image: Resource::new(PixelImage::new(vec![0u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
      delay_ms: 200,
    };
    let img = Image::from_parts(Vec::new(), 1, 1, LoopCount::Finite(2), vec![frame1, frame2]);

    assert_eq!(img.frame_count(), 2);
    assert!(img.is_animated());
    assert_eq!(img.loop_count(), LoopCount::Finite(2));
    assert_eq!(img.total_duration_ms(), 300);
  }

  #[test]
  fn frame_access() {
    let frame = DecodedFrame {
      image: Resource::new(PixelImage::new(vec![255u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
      delay_ms: 50,
    };
    let img = Image::from_parts(Vec::new(), 1, 1, LoopCount::Infinite, vec![frame]);

    // Valid index
    let f = img.frame(0);
    assert!(f.is_some());
    assert_eq!(f.unwrap().delay_ms, 50);

    // Out of bounds
    assert!(img.frame(1).is_none());
    assert!(img.frame(100).is_none());
  }

  #[test]
  fn first_frame() {
    let frame = DecodedFrame {
      image: Resource::new(PixelImage::new(vec![128u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
      delay_ms: 0,
    };
    let img = Image::from_parts(Vec::new(), 1, 1, LoopCount::Infinite, vec![frame]);

    let f = img.first_frame();
    assert_eq!(f.delay_ms, 0);
  }

  #[test]
  fn frame_iterator() {
    let frames = vec![
      DecodedFrame {
        image: Resource::new(PixelImage::new(vec![0u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
        delay_ms: 10,
      },
      DecodedFrame {
        image: Resource::new(PixelImage::new(vec![1u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
        delay_ms: 20,
      },
      DecodedFrame {
        image: Resource::new(PixelImage::new(vec![2u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
        delay_ms: 30,
      },
    ];
    let img = Image::from_parts(Vec::new(), 1, 1, LoopCount::Infinite, frames);

    let mut iter = img.frame_iter();
    assert_eq!(iter.len(), 3);

    assert_eq!(iter.next().unwrap().delay_ms, 10);
    assert_eq!(iter.len(), 2);

    assert_eq!(iter.next().unwrap().delay_ms, 20);
    assert_eq!(iter.next().unwrap().delay_ms, 30);
    assert!(iter.next().is_none());
  }

  #[test]
  fn global_frame_count() {
    // Infinite loop
    let frame = DecodedFrame {
      image: Resource::new(PixelImage::new(vec![0u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
      delay_ms: 0,
    };
    let img = Image::from_parts(Vec::new(), 1, 1, LoopCount::Infinite, vec![frame]);
    assert_eq!(img.global_frame_count(), None);

    // Finite loop: 3 frames * 2 loops = 6
    let frames: Vec<_> = (0..3)
      .map(|_| DecodedFrame {
        image: Resource::new(PixelImage::new(vec![0u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
        delay_ms: 0,
      })
      .collect();
    let img = Image::from_parts(Vec::new(), 1, 1, LoopCount::Finite(2), frames);
    assert_eq!(img.global_frame_count(), Some(6));
  }

  #[test]
  fn frame_by_global_idx_wrapping() {
    let frames = vec![
      DecodedFrame {
        image: Resource::new(PixelImage::new(vec![0u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
        delay_ms: 10,
      },
      DecodedFrame {
        image: Resource::new(PixelImage::new(vec![1u8; 4].into(), 1, 1, ColorFormat::Rgba8)),
        delay_ms: 20,
      },
    ];
    let img = Image::from_parts(Vec::new(), 1, 1, LoopCount::Finite(3), frames);

    // First loop
    assert_eq!(img.frame_by_global_idx(0).unwrap().delay_ms, 10);
    assert_eq!(img.frame_by_global_idx(1).unwrap().delay_ms, 20);
    // Second loop (wraps)
    assert_eq!(img.frame_by_global_idx(2).unwrap().delay_ms, 10);
    assert_eq!(img.frame_by_global_idx(3).unwrap().delay_ms, 20);
    // Third loop
    assert_eq!(img.frame_by_global_idx(4).unwrap().delay_ms, 10);
    assert_eq!(img.frame_by_global_idx(5).unwrap().delay_ms, 20);
  }

  #[test]
  fn debug_format() {
    let frame = DecodedFrame {
      image: Resource::new(PixelImage::new(vec![0u8; 16].into(), 2, 2, ColorFormat::Rgba8)),
      delay_ms: 0,
    };
    let img = Image::from_parts(Vec::new(), 2, 2, LoopCount::Infinite, vec![frame]);

    let debug = format!("{:?}", img);
    assert!(debug.contains("Image"));
    assert!(debug.contains("2x2"));
    assert!(debug.contains("1/1"));
    assert!(debug.contains("Infinite"));
  }

  // --- Widget Visual Tests ---
  // Note: Visual tests are in a separate module below with proper cfg
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod visual_tests {
  use ribir::{core::test_helper::*, material as ribir_material, prelude::*};
  use ribir_dev_helper::*;

  /// Create a test Image with a red/blue gradient pattern
  fn colored_image(width: u32, height: u32) -> Image {
    let size = (width * height * 4) as usize;
    let mut pixels = Vec::with_capacity(size);
    for y in 0..height {
      for x in 0..width {
        // Create a gradient: red increases left-to-right, blue increases top-to-bottom
        let r = ((x as f32 / width as f32) * 255.0) as u8;
        let g = 100;
        let b = ((y as f32 / height as f32) * 255.0) as u8;
        pixels.extend_from_slice(&[r, g, b, 255]);
      }
    }
    let frame = DecodedFrame {
      image: Resource::new(PixelImage::new(pixels.into(), width, height, ColorFormat::Rgba8)),
      delay_ms: 0,
    };
    Image::from_parts(Vec::new(), width, height, LoopCount::Infinite, vec![frame])
  }

  /// Create a test Resource<PixelImage> with a green/red gradient pattern
  fn colored_pixel_image(width: u32, height: u32) -> Resource<PixelImage> {
    let size = (width * height * 4) as usize;
    let mut pixels = Vec::with_capacity(size);
    for y in 0..height {
      for x in 0..width {
        // Create a different gradient: green increases left-to-right, red increases
        // top-to-bottom
        let r = ((y as f32 / height as f32) * 255.0) as u8;
        let g = ((x as f32 / width as f32) * 255.0) as u8;
        let b = 80;
        pixels.extend_from_slice(&[r, g, b, 255]);
      }
    }
    Resource::new(PixelImage::new(pixels.into(), width, height, ColorFormat::Rgba8))
  }

  widget_image_tests!(
    image_widget,
    WidgetTester::new(fn_widget! { @colored_image(100, 80) }).with_wnd_size(Size::new(120., 100.)),
  );

  widget_image_tests!(
    pixel_image_widget,
    WidgetTester::new(fn_widget! { @colored_pixel_image(80, 60) })
      .with_wnd_size(Size::new(100., 80.)),
  );
}
