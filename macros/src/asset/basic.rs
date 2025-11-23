use proc_macro2::TokenStream;
use quote::quote;

use super::{Asset, AssetContext};

pub(crate) struct BinaryAsset;

impl Asset for BinaryAsset {
  fn generate(&self, ctx: &AssetContext) -> syn::Result<TokenStream> {
    if ctx.embed {
      let path = &ctx.input_path;
      Ok(quote! { include_bytes!(#path).to_vec() })
    } else {
      ctx.copy_input_to_output()?;
      let path_tokens = ctx.runtime_path_tokens();
      let panic_msg = ctx.panic_msg("read binary");
      Ok(quote! {
        {
          let asset_path = #path_tokens;
          std::fs::read(&asset_path)
            .unwrap_or_else(|e| panic!("{}: {}. Asset path: {:?}", #panic_msg, e, asset_path))
        }
      })
    }
  }
}

pub(crate) struct TextAsset;

impl Asset for TextAsset {
  fn generate(&self, ctx: &AssetContext) -> syn::Result<TokenStream> {
    if ctx.embed {
      let path = &ctx.input_path;
      Ok(quote! { include_str!(#path).to_string() })
    } else {
      ctx.copy_input_to_output()?;
      let path_tokens = ctx.runtime_path_tokens();
      let panic_msg = ctx.panic_msg("read text");
      Ok(quote! {
        {
          let asset_path = #path_tokens;
          std::fs::read_to_string(&asset_path)
            .unwrap_or_else(|e| panic!("{}: {}. Asset path: {:?}", #panic_msg, e, asset_path))
        }
      })
    }
  }
}
