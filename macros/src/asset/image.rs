use std::fs;

use image::{AnimationDecoder, GenericImageView, ImageFormat, codecs::gif::GifDecoder};
use proc_macro2::TokenStream;
use quote::quote;
use webp_animation::prelude::*;

use super::{Asset, AssetContext};

pub(crate) struct ImageAsset;

impl Asset for ImageAsset {
  fn process(&self, ctx: &AssetContext) -> syn::Result<Option<Vec<u8>>> {
    let data = fs::read(&ctx.abs_input).map_err(|e| ctx.error(format!("Read failed: {e}")))?;
    let format =
      image::guess_format(&data).map_err(|e| ctx.error(format!("Unknown format: {e}")))?;

    let webp_data = match format {
      ImageFormat::WebP => data, // Already WebP, no conversion needed
      ImageFormat::Gif => encode_gif(&data, ctx)?,
      _ => encode_static(&data, format, ctx)?,
    };

    Ok(Some(webp_data))
  }

  fn output_extension(&self) -> Option<&str> { Some("webp") }

  fn load_expr(&self, data_expr: TokenStream) -> TokenStream {
    quote! { Image::new(#data_expr).expect("Invalid WebP") }
  }
}

/// Encode static image to WebP.
fn encode_static(data: &[u8], format: ImageFormat, ctx: &AssetContext) -> syn::Result<Vec<u8>> {
  let img = image::load_from_memory_with_format(data, format)
    .map_err(|e| ctx.error(format!("Decode: {e}")))?;
  let (w, h) = img.dimensions();

  let mut enc = Encoder::new((w, h)).map_err(|e| ctx.error(format!("Encoder init: {e}")))?;
  enc
    .add_frame(&img.to_rgba8(), 0)
    .map_err(|e| ctx.error(format!("Frame encode: {e}")))?;
  enc
    .finalize(0)
    .map(|d| d.to_vec())
    .map_err(|e| ctx.error(format!("Finalize: {e}")))
}

/// Encode animated GIF to animated WebP.
fn encode_gif(data: &[u8], ctx: &AssetContext) -> syn::Result<Vec<u8>> {
  let decoder = GifDecoder::new(std::io::Cursor::new(data))
    .map_err(|e| ctx.error(format!("GIF decode: {e}")))?;
  let frames: Vec<_> = decoder
    .into_frames()
    .collect::<Result<_, _>>()
    .map_err(|e| ctx.error(format!("Frame decode: {e}")))?;

  if frames.is_empty() {
    return Err(ctx.error("GIF has no frames"));
  }

  let (w, h) = frames[0].buffer().dimensions();
  let mut enc = Encoder::new((w, h)).map_err(|e| ctx.error(format!("Encoder init: {e}")))?;

  let mut ts = 0i32;
  for f in &frames {
    enc
      .add_frame(f.buffer().as_raw(), ts)
      .map_err(|e| ctx.error(format!("Frame encode: {e}")))?;
    let (num, denom) = f.delay().numer_denom_ms();
    ts += if denom == 0 { 100 } else { (num / denom) as i32 };
  }

  enc
    .finalize(ts)
    .map(|d| d.to_vec())
    .map_err(|e| ctx.error(format!("Finalize: {e}")))
}
