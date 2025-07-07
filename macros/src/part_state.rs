use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
  AngleBracketedGenericArguments, Expr, Member, Result, Token, parenthesized,
  parse::{Parse, ParseStream},
  punctuated::Punctuated,
  token::Paren,
};

use crate::{
  dollar_macro::{OriginExpr, StateExpr},
  symbol_process::*,
};

pub fn gen_part_writer(input: TokenStream, refs_ctx: Option<&mut DollarRefsCtx>) -> TokenStream {
  match syn::parse2::<PartialPartState>(input) {
    Ok(PartialPartState { id, part_state, .. }) => {
      let host = part_state.host_tokens(DollarUsedInfo::Writer, refs_ctx);
      let PartState { and_token, mutability, state, dot, part_expr, tail_dot, tail_expr } =
        part_state;
      let id = id
        .map(|id| quote! { #id.into() })
        .unwrap_or(quote! { PartialId::any() });
      let tokens = quote_spanned! { state.span() =>
        #host #dot part_writer(
          #id,
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
        #host #dot part_reader(
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
        #host #dot part_watcher(
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

struct PartialPartState {
  id: Option<Expr>,
  part_state: PartState,
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

impl Parse for PartialPartState {
  fn parse(input: ParseStream) -> Result<Self> {
    let part_state = input.parse();
    if let Ok(part_state) = part_state {
      return Ok(Self { id: None, part_state });
    }

    let id = input.parse()?;
    input.parse::<Token![,]>()?;
    let part_state = input.parse()?;
    Ok(PartialPartState { id: Some(id), part_state })
  }
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
    let mut tokens = quote! {};
    if let Some(refs_ctx) = refs_ctx {
      let state_expr = StateExpr::new(self.state.clone(), OriginExpr::Var(self.state.clone()));
      if !refs_ctx.is_capture_var(&self.state) {
        self.state.to_tokens(&mut tokens);
      } else {
        state_expr.name.to_tokens(&mut tokens);
      }
      refs_ctx.add_dollar_ref(DollarRef { state_expr, used });
    } else {
      self.state.to_tokens(&mut tokens);
    }

    tokens
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
