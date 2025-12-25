use proc_macro2::TokenStream;
use quote::quote;

use super::{Asset, AssetContext};

pub(crate) struct BinaryAsset;

impl Asset for BinaryAsset {
  fn process(&self, _ctx: &AssetContext) -> syn::Result<Option<Vec<u8>>> {
    // No processing needed, use original data
    Ok(None)
  }

  fn load_expr(&self, data_expr: TokenStream) -> TokenStream {
    quote! { #data_expr.into_owned() }
  }
}

pub(crate) struct TextAsset;

impl Asset for TextAsset {
  fn process(&self, _ctx: &AssetContext) -> syn::Result<Option<Vec<u8>>> {
    // No processing needed, use original data
    Ok(None)
  }

  fn load_expr(&self, data_expr: TokenStream) -> TokenStream {
    quote! { String::from_utf8(#data_expr.into_owned()).expect("Invalid UTF-8") }
  }
}
