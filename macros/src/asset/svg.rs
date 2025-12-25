use proc_macro2::TokenStream;
use quote::quote;

use super::{Asset, AssetContext};

pub(crate) struct SvgAsset {
  pub(crate) inherit_fill: bool,
  pub(crate) inherit_stroke: bool,
}

impl Asset for SvgAsset {
  fn process(&self, ctx: &AssetContext) -> syn::Result<Option<Vec<u8>>> {
    let svg = ribir_painter::Svg::open(&ctx.abs_input, self.inherit_fill, self.inherit_stroke)
      .map_err(|e| ctx.error(format!("SVG open failed: {e}")))?;
    let serialized = svg
      .serialize()
      .map_err(|e| ctx.error(format!("SVG serialize failed: {e}")))?;
    Ok(Some(serialized.into_bytes()))
  }

  fn load_expr(&self, data_expr: TokenStream) -> TokenStream {
    quote! {
      Svg::deserialize(
        &String::from_utf8(#data_expr.into_owned()).expect("Invalid UTF-8")
      ).expect("Failed to deserialize SVG")
    }
  }

  fn params_hash(&self) -> Option<String> {
    let hash = (self.inherit_fill as u8) | ((self.inherit_stroke as u8) << 1);
    Some(format!("{:x}", hash))
  }
}
