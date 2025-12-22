use proc_macro2::TokenStream;
use quote::quote;

use super::{Asset, AssetContext};

pub(crate) struct SvgAsset {
  pub(crate) inherit_fill: bool,
  pub(crate) inherit_stroke: bool,
}

impl Asset for SvgAsset {
  fn generate(&self, ctx: &AssetContext) -> syn::Result<TokenStream> {
    // Stable dependency tracking: make the caller crate depend on the SVG source.
    // We use the resolved absolute path so changes always trigger recompilation.
    let abs_input = ctx.abs_input.to_string_lossy().into_owned();

    // The data source is always the processed output file
    let load_expr = if ctx.embed {
      // For embedding, always process (no caching benefit since it's in the binary)
      let compressed_data = self.process_svg(ctx)?;
      quote! { #compressed_data }
    } else {
      // For runtime loading: only process if output needs update
      if ctx.needs_update() {
        let compressed_data = self.process_svg(ctx)?;
        ctx.write_to_output(compressed_data.as_bytes())?;
      }
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
        const _: &[u8] = include_bytes!(#abs_input);

        Svg::deserialize(#load_expr)
          .unwrap_or_else(|e| panic!("{}: {}", #deserialize_msg, e))
      }
    })
  }

  fn params_hash(&self) -> Option<String> {
    // Encode SVG processing parameters into a short hash
    // Format: 2 bits encoded as hex (f=fill, s=stroke)
    let hash = (self.inherit_fill as u8) | ((self.inherit_stroke as u8) << 1);
    Some(format!("{:x}", hash))
  }
}

impl SvgAsset {
  fn process_svg(&self, ctx: &AssetContext) -> syn::Result<String> {
    ribir_painter::Svg::open(&ctx.abs_input, self.inherit_fill, self.inherit_stroke)
      .and_then(|svg| svg.serialize())
      .map_err(|e| {
        syn::Error::new(
          ctx.input_span,
          format!("Failed to compress SVG file '{}': {}", ctx.input_path, e),
        )
      })
  }
}
