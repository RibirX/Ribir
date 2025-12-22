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
      ctx.copy_to_output()?;
      // Stable dependency tracking: make the caller crate depend on the source file.
      // We use the resolved absolute path so changes always trigger recompilation.
      let abs_input = ctx.abs_input.to_string_lossy().into_owned();
      let path_tokens = ctx.runtime_path_tokens();
      let panic_msg = ctx.panic_msg("read binary");
      Ok(quote! {
        {
          const _: &[u8] = include_bytes!(#abs_input);

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
      ctx.copy_to_output()?;
      // Stable dependency tracking: make the caller crate depend on the source file.
      // We use the resolved absolute path so changes always trigger recompilation.
      let abs_input = ctx.abs_input.to_string_lossy().into_owned();
      let path_tokens = ctx.runtime_path_tokens();
      let panic_msg = ctx.panic_msg("read text");
      Ok(quote! {
        {
          const _: &[u8] = include_bytes!(#abs_input);

          let asset_path = #path_tokens;
          std::fs::read_to_string(&asset_path)
            .unwrap_or_else(|e| panic!("{}: {}. Asset path: {:?}", #panic_msg, e, asset_path))
        }
      })
    }
  }
}
