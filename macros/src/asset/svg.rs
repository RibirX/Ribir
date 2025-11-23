use proc_macro2::TokenStream;
use quote::quote;

use super::{Asset, AssetContext};

pub(crate) struct SvgAsset {
  pub(crate) inherit_fill: bool,
  pub(crate) inherit_stroke: bool,
}

impl Asset for SvgAsset {
  fn generate(&self, ctx: &AssetContext) -> syn::Result<TokenStream> {
    // SVG always requires processing (compression/serialization), so we always
    // write to output, even if embedding.
    let compressed_data =
      ribir_painter::Svg::open(&ctx.abs_input, self.inherit_fill, self.inherit_stroke)
        .and_then(|svg| svg.serialize())
        .map_err(|e| {
          syn::Error::new(
            ctx.input_span,
            format!("Failed to compress SVG file '{}': {}", ctx.input_path, e),
          )
        })?;

    // The data source is always the processed output file
    let load_expr = if ctx.embed {
      quote! { #compressed_data }
    } else {
      ctx
        .write_output(compressed_data.as_bytes())
        .expect("Failed to write SVG data");
      let path_tokens = ctx.runtime_path_tokens();
      let panic_msg = ctx.panic_msg("read SVG");
      quote! {
        &std::fs::read_to_string(&#path_tokens)
          .unwrap_or_else(|e| panic!("{}: {}", #panic_msg, e))
      }
    };

    let deserialize_msg = format!("Failed to deserialize SVG asset '{}'", ctx.relative_output);

    Ok(quote! {
      {
        Svg::deserialize(#load_expr)
          .unwrap_or_else(|e| panic!("{}: {}", #deserialize_msg, e))
      }
    })
  }
}
