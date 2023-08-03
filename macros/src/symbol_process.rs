use crate::widget_macro::{
  ribir_suffix_variable, WIDGET_OF_BUILTIN_FIELD, WIDGET_OF_BUILTIN_METHOD,
};
use inflector::Inflector;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use smallvec::SmallVec;
use syn::{
  fold::Fold, parse_quote, parse_quote_spanned, spanned::Spanned, Expr, ExprField, ExprMacro,
  ExprMethodCall, Macro, Member, Stmt,
};

pub const KW_DOLLAR_STR: &str = "_dollar_ಠ_ಠ";
pub const KW_CTX: &str = "ctx";
pub const KW_RDL: &str = "rdl";
pub use tokens_pre_process::*;

pub mod kw {
  syn::custom_keyword!(_dollar_ಠ_ಠ);
  syn::custom_keyword!(rdl);
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct DollarRef {
  pub name: Ident,
  pub value: Expr,
}
#[derive(Default)]
pub struct DollarRefs {
  refs: SmallVec<[DollarRef; 1]>,
  ctx_used: bool,
  fold_refs_inline: bool,
}

mod tokens_pre_process {

  use proc_macro::{TokenTree, *};
  use quote::quote_spanned;

  use super::KW_DOLLAR_STR;
  use crate::symbol_process::KW_RDL;

  fn rdl_syntax_err<T>(span: Span) -> Result<T, TokenStream> {
    let err_token = quote_spanned! { span.into() =>
      compile_error!("Syntax Error: use `@` to declare object, must be: \n \
        1. `@ XXX { ... }`, declare a new `XXX` type object;\n \
        2. `@ $parent { ... }`, declare a variable as parent;\n \
        3. `@ { ... } `, declare an object by an expression.")
    };
    Err(err_token.into())
  }

  fn dollar_err<T>(span: Span) -> Result<T, TokenStream> {
    let err_token = quote_spanned! { span.into() =>
      compile_error!("Syntax error: expected an identifier after `$`")
    };
    Err(err_token.into())
  }

  /// Convert `@` and `$` symbol to a `rdl!` or `_dollar_ಠ_ಠ!` macro, make it
  /// conform to Rust syntax
  pub fn symbol_to_macro(input: TokenStream) -> Result<TokenStream, TokenStream> {
    let mut iter = input.into_iter();
    let mut tokens = vec![];

    loop {
      match iter.next() {
        Some(TokenTree::Punct(at))
          // maybe rust identify bind syntax, `identify @`
          if at.as_char() == '@' && !matches!(tokens.last(), Some(TokenTree::Ident(_))) =>
        {
          tokens.push(TokenTree::Ident(Ident::new(KW_RDL, at.span())));
          tokens.push(TokenTree::Punct(Punct::new('!', Spacing::Alone)));

          let body = match iter.next() {
            // declare a new widget: `@ SizedBox { ... }`
            Some(TokenTree::Ident(name)) => {
              let Some(TokenTree::Group(body))  =  iter.next() else {
                return rdl_syntax_err(at.span().join(name.span()).unwrap())
              };
              let tokens = TokenStream::from_iter([TokenTree::Ident(name), TokenTree::Group(body)]);
              Group::new(Delimiter::Brace, tokens)
            }
            // declare a variable widget as parent,  `@ $var { ... }`
            Some(TokenTree::Punct(dollar)) if dollar.as_char() == '$' => {
              if let Some(TokenTree::Ident(var)) = iter.next() {
                let Some(TokenTree::Group(body))  =  iter.next() else {
                  let span = at.span().join(dollar.span()).unwrap().join(var.span()).unwrap();
                  return rdl_syntax_err(span)
                };
                let tokens = TokenStream::from_iter([
                  TokenTree::Punct(dollar),
                  TokenTree::Ident(var),
                  TokenTree::Group(body),
                ]);
                Group::new(Delimiter::Brace, tokens)
              } else {
                return dollar_err(dollar.span());
              }
            }
            // declare a expression widget  `@ { ... }`
            Some(TokenTree::Group(g)) => g,
            n => {
              let mut span = at.span();
              if let Some(n) = n {
                span = span.join(n.span()).unwrap()
              }
              return rdl_syntax_err(span);
            }
          };
          tokens.push(TokenTree::Group(body));
        }
        Some(TokenTree::Punct(p)) if p.as_char() == '$' => {
          match iter.next() {
            Some(TokenTree::Ident(name)) => {
              tokens.push(TokenTree::Ident(Ident::new(KW_DOLLAR_STR, p.span())));
              tokens.push(TokenTree::Punct(Punct::new('!', Spacing::Alone)));
              let span = name.span();
              let mut g = Group::new(Delimiter::Parenthesis, TokenTree::Ident(name).into());
              g.set_span(span);
              tokens.push(TokenTree::Group(g));
            }
            Some(t) => return dollar_err(t.span()),
            None => return dollar_err(p.span()),
          };
        }

        Some(TokenTree::Group(mut g)) => {
          // not process symbol in macro, because it's maybe as part of the macro syntax.
          if !in_macro(&tokens) {
            let mut n = Group::new(g.delimiter(), symbol_to_macro(g.stream())?);
            n.set_span(g.span());
            g = n;
          }

          tokens.push(TokenTree::Group(g));
        }
        Some(t) => tokens.push(t),
        None => break,
      };
    }
    Ok(tokens.into_iter().collect())
  }

  fn in_macro(tokens: &[TokenTree]) -> bool {
    let [.., TokenTree::Ident(_), TokenTree::Punct(p)] = tokens else {
    return  false;
  };
    p.as_char() == '!'
  }
}

impl Fold for DollarRefs {
  fn fold_expr_field(&mut self, mut i: ExprField) -> ExprField {
    let ExprField { base, member, .. } = &mut i;
    if let Member::Named(member) = member {
      if let Some(builtin_ty) = WIDGET_OF_BUILTIN_FIELD.get(member.to_string().as_str()) {
        self.replace_builtin_ident(
          &mut *base,
          &builtin_ty.to_snake_case(),
          self.fold_refs_inline,
        );
      }
    }

    syn::fold::fold_expr_field(self, i)
  }

  fn fold_expr_method_call(&mut self, mut i: ExprMethodCall) -> ExprMethodCall {
    if let Some(builtin_ty) = WIDGET_OF_BUILTIN_METHOD.get(i.method.to_string().as_str()) {
      self.replace_builtin_ident(
        &mut i.receiver,
        &builtin_ty.to_snake_case(),
        self.fold_refs_inline,
      );
    }

    syn::fold::fold_expr_method_call(self, i)
  }
  fn fold_expr(&mut self, i: Expr) -> Expr {
    match i {
      Expr::Macro(e @ ExprMacro { .. }) => {
        if let Some(name) = dollar_macro_inner_ident(&e.mac) {
          let value: Expr = if self.fold_refs_inline {
            parse_quote_spanned! { name.span() => #name.state_ref() }
          } else {
            parse_quote! { #name }
          };
          self.refs.push(DollarRef { name, value: value.clone() });
          value
        } else {
          self.ctx_used = e.mac.path.is_ident(KW_RDL) || e.mac.path.is_ident(KW_CTX);
          Expr::Macro(self.fold_expr_macro(e))
        }
      }
      Expr::Closure(c) if c.capture.is_some() => {
        let mut closure_refs = DollarRefs::default();
        let mut c = closure_refs.fold_expr_closure(c);

        if !closure_refs.is_empty() || closure_refs.ctx_used {
          closure_refs.dedup();

          let c_refs = closure_refs.refs.iter().map(CaptureRef);
          let body = &mut *c.body;
          if let Expr::Block(block) = body {
            let refs_stmts = Stmt::Expr(Expr::Verbatim(quote! { #(#c_refs;)*}));
            block.block.stmts.insert(0, refs_stmts);
          } else {
            *body = Expr::Verbatim(quote_spanned!(body.span() => { #(#c_refs)* #body }));
          }

          if closure_refs.ctx_used {
            *body = Expr::Verbatim(quote_spanned!(body.span() =>
              _ctx_handle
                .with_ctx(|ctx!(): &'_ BuildCtx<'_>| #body )
                .expect("The `BuildCtx` is not available.")
            ));
          }

          let captures = closure_refs.capture_state_tokens();
          let handle = closure_refs
            .ctx_used
            .then(|| quote_spanned! { c.span() => let _ctx_handle = ctx!().handle(); });
          Expr::Verbatim(quote_spanned!(c.span() => {
            #captures
            #handle
            #c
          }))
        } else {
          Expr::Closure(c)
        }
      }
      _ => syn::fold::fold_expr(self, i),
    }
  }
}

impl ToTokens for DollarRef {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let DollarRef { name, value } = self;
    quote_spanned! { value.span() =>
      let mut #name = #value.state_ref();
    }
    .to_tokens(tokens);
  }
}
impl ToTokens for DollarRefs {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    for dollar_ref in &self.refs {
      dollar_ref.to_tokens(tokens);
    }
  }
}

impl DollarRefs {
  pub fn new(fold_refs_inline: bool) -> Self {
    Self {
      fold_refs_inline,
      ..Default::default()
    }
  }

  pub fn used_ctx(&self) -> bool { self.ctx_used }

  pub fn dedup(&mut self) { self.refs.dedup(); }

  pub fn capture_state_tokens(&self) -> TokenStream {
    let mut tokens = quote! {};
    for dollar_ref in &self.refs {
      let DollarRef { name, value } = dollar_ref;
      quote_spanned! { value.span() =>
        let #name = #value.clone_stateful();
      }
      .to_tokens(&mut tokens);
    }
    tokens
  }

  pub fn upstream_tokens(&self) -> TokenStream {
    match self.len() {
      0 => quote! {},
      1 => {
        let DollarRef { name, value } = &self.refs[0];
        quote_spanned! { value.span() => #name.modifies() }
      }
      _ => {
        let upstream = self.iter().map(|DollarRef { name, .. }| {
          quote! {  #name.modifies() }
        });
        quote! { observable::from_iter([#(#upstream),*]).merge_all(usize::MAX) }
      }
    }
  }

  fn replace_builtin_ident(
    &mut self,
    caller: &mut Expr,
    builtin_member: &str,
    inline: bool,
  ) -> Option<&DollarRef> {
    let e = match caller {
      Expr::MethodCall(ExprMethodCall { receiver, method, args, .. })
        if args.is_empty() && (method == "shallow" || method == "silent") =>
      {
        &mut **receiver
      }
      e => e,
    };

    let Expr::Macro(m) = e else { return None };
    let host = dollar_macro_inner_ident(&m.mac)?;
    let builtin_name = ribir_suffix_variable(&host, builtin_member);
    let builtin_member = Ident::new(builtin_member, host.span());
    if inline {
      *e = parse_quote_spanned! { host.span() => #host.#builtin_member(ctx!()).state_ref() };
    } else {
      *e = parse_quote!(#builtin_name);
    }
    self.refs.push(DollarRef {
      name: builtin_name,
      value: parse_quote! { #host.#builtin_member(ctx!()) },
    });
    self.last()
  }
}

/// A builtin widget has different references declare in the capture closure or
/// not, because the capture closure already clone the builtin widget.
pub struct CaptureRef<'a>(pub &'a DollarRef);

impl<'a> ToTokens for CaptureRef<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let CaptureRef(DollarRef { name, .. }) = self;
    quote_spanned! { name.span() => let mut #name = #name.state_ref(); }.to_tokens(tokens)
  }
}

fn dollar_macro_inner_ident(mac: &Macro) -> Option<Ident> {
  mac.path.is_ident(KW_DOLLAR_STR).then(|| {
    let tokens = &mac.tokens;
    parse_quote!(#tokens)
  })
}

impl std::ops::Deref for DollarRefs {
  type Target = [DollarRef];
  fn deref(&self) -> &Self::Target { &self.refs }
}
