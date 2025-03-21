use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote_spanned};
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

pub fn gen_part_writer(input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  match syn::parse2::<PartState>(input) {
    Ok(part) => {
      let host = part.host_tokens(DollarUsedInfo::Writer, refs_ctx);
      let PartState { and_token, mutability, state, dot, part_expr, tail_dot, tail_expr } = part;
      let tokens = quote_spanned! { state.span() =>
        #host #dot map_writer(
          |w| PartMut::new(#and_token #mutability w #dot #part_expr #tail_dot #tail_expr)
        )
      };
      tokens
    }
    Err(err) => err.to_compile_error(),
  }
}

pub fn gen_split_writer(input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  match syn::parse2::<PartState>(input) {
    Ok(part) => {
      let host = part.host_tokens(DollarUsedInfo::Writer, refs_ctx);
      let PartState { and_token, mutability, state, dot, part_expr, tail_dot, tail_expr } = part;
      let tokens = quote_spanned! { state.span() =>
        #host #dot split_writer(
          |w| PartMut::new(#and_token #mutability w #dot #part_expr #tail_dot #tail_expr)
        )
      };
      tokens
    }
    Err(err) => err.to_compile_error(),
  }
}

pub fn gen_part_reader(input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  match syn::parse2::<PartState>(input) {
    Ok(part) => {
      let host = part.host_tokens(DollarUsedInfo::Reader, refs_ctx);
      let PartState { and_token, mutability, state, dot, part_expr, tail_dot, tail_expr } = part;
      let tokens = quote_spanned! { state.span() =>
        #host #dot map_reader(
          |r| PartRef::new(#and_token #mutability r #dot #part_expr #tail_dot #tail_expr)
        )
      };
      tokens
    }
    Err(err) => err.to_compile_error(),
  }
}

pub fn gen_part_watcher(input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  match syn::parse2::<PartState>(input) {
    Ok(part) => {
      let host = part.host_tokens(DollarUsedInfo::Watcher, refs_ctx);
      let PartState { and_token, mutability, state, dot, part_expr, tail_dot, tail_expr } = part;
      let tokens = quote_spanned! { state.span() =>
        #host #dot map_watcher(
          |r| PartRef::new(#and_token #mutability r #dot #part_expr #tail_dot #tail_expr)
        )
      };
      tokens
    }
    Err(err) => err.to_compile_error(),
  }
}

struct PartState {
  and_token: Option<Token![&]>,
  mutability: Option<Token![mut]>,
  state: Ident,
  dot: Token![.],
  part_expr: PartExpr,
  tail_dot: Option<Token![.]>,
  tail_expr: Option<Expr>,
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

impl Parse for PartState {
  fn parse(input: ParseStream) -> Result<Self> {
    let and_token = input.parse()?;
    let mutability = input.parse()?;
    let state = if input.peek(Token![self]) {
      let this = input.parse::<Token![self]>()?;
      Ident::from(this)
    } else {
      input.parse()?
    };
    let dot = input.parse()?;

    let part_expr = if input.peek2(Token![::]) || input.peek2(Paren) {
      let method = input.parse()?;
      let turbofish = if input.peek(Token![::]) {
        Some(AngleBracketedGenericArguments::parse_turbofish(input)?)
      } else {
        None
      };
      let content;
      PartExpr::Method {
        method,
        turbofish,
        paren_token: parenthesized!(content in input),
        args: content.parse_terminated(Expr::parse, Token![,])?,
      }
    } else {
      PartExpr::Member(input.parse()?)
    };

    let tail_dot = input.parse()?;
    let tail_expr = if input.is_empty() { None } else { Some(input.parse()?) };
    Ok(Self { and_token, mutability, state, dot, part_expr, tail_dot, tail_expr })
  }
}

impl PartState {
  fn host_tokens(&self, used: DollarUsedInfo, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
    if let Some(refs_ctx) = refs_ctx {
      let info = self.state_info(refs_ctx, used);
      let host = if info.builtin.is_some() {
        refs_ctx.builtin_host_tokens(&info)
      } else {
        self.state.to_token_stream()
      };
      refs_ctx.add_dollar_ref(info);
      host
    } else {
      let ctx = DollarRefsCtx::top_level();
      let info = self.state_info(&ctx, used);
      info.real_state_tokens()
    }
  }

  fn state_info(&self, refs_ctx: &DollarRefsCtx, used: DollarUsedInfo) -> DollarRef {
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
      refs_ctx.builtin_dollar_ref(self.state.clone(), info, used)
    } else {
      DollarRef { name: self.state.clone(), builtin: None, used }
    }
  }
}

impl ToTokens for PartExpr {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      PartExpr::Member(member) => member.to_tokens(tokens),
      PartExpr::Method { method, turbofish, paren_token, args } => {
        method.to_tokens(tokens);
        turbofish.to_tokens(tokens);
        paren_token.surround(tokens, |tokens| args.to_tokens(tokens));
      }
    }
  }
}
