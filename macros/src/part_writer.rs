use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
  AngleBracketedGenericArguments, Expr, Member, Result, Token, parenthesized,
  parse::{Parse, ParseStream},
  punctuated::Punctuated,
  token::Paren,
};

use crate::{
  symbol_process::{DollarRef, DollarRefsCtx, DollarUsedInfo},
  variable_names::{BUILTIN_INFOS, BuiltinMemberType},
};

pub fn gen_code(input: TokenStream, refs_ctx: &mut DollarRefsCtx) -> TokenStream {
  match syn::parse2::<PartWriter>(input) {
    Ok(part) => {
      let info = part.writer_info(refs_ctx);
      let tokens = part.gen_tokens(&info, refs_ctx);
      refs_ctx.add_dollar_ref(info);
      tokens
    }
    Err(err) => err.to_compile_error(),
  }
}

struct PartWriter {
  and_token: Option<Token![&]>,
  mutability: Option<Token![mut]>,
  writer: Ident,
  dot: Token![.],
  part_expr: PartExpr,
}

enum PartExpr {
  Member(Member),
  Method {
    method: Ident,
    turbofish: Option<AngleBracketedGenericArguments>,
    paren_token: Paren,
    args: Punctuated<Expr, Token![,]>,
  },
}

impl Parse for PartWriter {
  fn parse(input: ParseStream) -> Result<Self> {
    let and_token = input.parse()?;
    let mutability = input.parse()?;
    let writer = input.parse()?;
    let dot = input.parse()?;

    let part_expr = if input.peek(syn::LitInt) {
      PartExpr::Member(input.parse()?)
    } else {
      let name = input.parse::<Ident>()?;
      if input.is_empty() {
        PartExpr::Member(Member::Named(name))
      } else {
        let turbofish = if input.peek(Token![::]) {
          Some(AngleBracketedGenericArguments::parse_turbofish(input)?)
        } else {
          None
        };
        let content;
        PartExpr::Method {
          method: name,
          turbofish,
          paren_token: parenthesized!(content in input),
          args: content.parse_terminated(Expr::parse, Token![,])?,
        }
      }
    };
    Ok(Self { and_token, mutability, writer, dot, part_expr })
  }
}

impl PartWriter {
  fn writer_info(&self, refs_ctx: &DollarRefsCtx) -> DollarRef {
    let builtin_info = match &self.part_expr {
      PartExpr::Member(Member::Named(member)) => BUILTIN_INFOS
        .get(&member.to_string())
        .filter(|info| info.mem_ty == BuiltinMemberType::Field),
      PartExpr::Method { method, .. } => BUILTIN_INFOS
        .get(&method.to_string())
        .filter(|info| info.mem_ty == BuiltinMemberType::Method),
      _ => None,
    };
    if let Some(info) = builtin_info {
      refs_ctx.builtin_dollar_ref(self.writer.clone(), info, DollarUsedInfo::Writer)
    } else {
      DollarRef { name: self.writer.clone(), builtin: None, used: DollarUsedInfo::Writer }
    }
  }

  fn gen_tokens(&self, writer_info: &DollarRef, refs_ctx: &DollarRefsCtx) -> TokenStream {
    let Self { and_token, mutability, writer, dot, part_expr } = self;
    let part_expr = match part_expr {
      PartExpr::Member(member) => member.to_token_stream(),
      PartExpr::Method { method, turbofish, paren_token, args } => {
        let mut tokens = quote! {};
        method.to_tokens(&mut tokens);
        turbofish.to_tokens(&mut tokens);
        paren_token.surround(&mut tokens, |tokens| args.to_tokens(tokens));
        tokens
      }
    };
    let host = if writer_info.builtin.is_some() {
      refs_ctx.builtin_host_tokens(writer_info)
    } else {
      writer.to_token_stream()
    };

    quote_spanned! { writer.span() =>
      #host #dot map_writer(|w| PartData::from_ref_mut(#and_token #mutability w #dot #part_expr))
    }
  }
}
