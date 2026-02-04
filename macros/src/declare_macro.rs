use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Result;

mod codegen;
mod ir;
mod parser;

use codegen::CodegenContext;
use ir::Declarer;

const DECLARE_ATTR: &str = "declare";

pub(crate) fn declare_macro(stt: &mut syn::ItemStruct, is_attr: bool) -> Result<TokenStream> {
  let declarer = Declarer::new(stt)?;
  let ctx = CodegenContext::new(&declarer);

  let mut tokens = ctx.generate();

  if is_attr || declarer.simple {
    declarer.original.to_tokens(&mut tokens);
  }

  Ok(tokens)
}
