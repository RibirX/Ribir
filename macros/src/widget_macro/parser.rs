//! mod parse the `widget!` macro.
use proc_macro2::TokenStream;
use quote::ToTokens;
use smallvec::{smallvec, SmallVec};
use std::collections::HashSet;
use syn::{
  braced, parenthesized,
  parse::{Parse, ParseStream},
  parse_quote,
  punctuated::Punctuated,
  spanned::Spanned,
  token::{Brace, Colon, Colon2, Comma, Dot, FatArrow, Paren},
  Block, Expr, Ident, Path, Result,
};

use super::TrackExpr;

pub mod kw {
  syn::custom_keyword!(states);
  syn::custom_keyword!(init);
  syn::custom_keyword!(finally);
  syn::custom_keyword!(DynWidget);
  syn::custom_keyword!(id);
  syn::custom_keyword!(Animate);
  syn::custom_keyword!(Transition);
  syn::custom_keyword!(transition);
  syn::custom_punctuation!(AssignColon, :=);
}

pub struct MacroSyntax {
  pub init: Option<Init>,
  pub states: Option<States>,
  pub widget: DeclareWidget,
  pub items: Vec<Item>,
  pub finally: Option<Finally>,
}

pub struct States {
  _states_token: kw::states,
  _brace: Brace,
  pub states: Vec<StateField>,
}

pub struct Init {
  _init_token: kw::init,
  pub ctx_name: Option<Ident>,
  _fat_arrow: Option<FatArrow>,
  pub block: Block,
}

pub struct Finally {
  _finally_token: kw::finally,
  pub ctx_name: Option<Ident>,
  _fat_arrow: Option<FatArrow>,
  pub block: Block,
}

#[derive(Clone, Debug)]
pub enum FieldColon {
  Colon(Colon),
  AssignColon(kw::AssignColon),
}
#[derive(Debug)]
pub struct StateField {
  pub(crate) member: Ident,
  pub(crate) colon_token: Option<Colon>,
  pub(crate) expr: Expr,
}

#[derive(Debug)]
pub enum DeclareWidget {
  /// Declare widget as struct literal.
  Literal {
    ty: Path,
    brace: Brace,
    fields: Punctuated<DeclareField, Comma>,
    children: Vec<DeclareWidget>,
  },
  /// Declare a widget use a path.
  Path(Path),
  /// Declare a widget across widget construct call, only as a leaf declare.
  /// `X::new(...)`
  Call(ConstructCall),
}

#[derive(Debug)]
pub struct ConstructCall {
  path: Path,
  paren: Paren,
  args: Punctuated<Expr, Comma>,
}
#[derive(Debug)]
pub struct Id {
  pub id: kw::id,
  pub colon: Colon,
  pub name: Ident,
  pub tail_comma: Option<Comma>,
}

#[derive(Clone, Debug)]
pub struct DeclareField {
  pub member: Ident,
  pub colon: Option<FieldColon>,
  pub expr: Expr,
}

pub struct DeclareSingle {
  pub ty: Path,
  pub brace: Brace,
  pub fields: Punctuated<DeclareField, Comma>,
}
pub enum Item {
  TransProps(TransProps),
  Transition(DeclareSingle),
  Animate(DeclareSingle),
}

pub struct TransProps {
  pub transition: kw::transition,
  pub props: SmallVec<[Expr; 1]>,
  pub brace: Brace,
  pub fields: Punctuated<DeclareField, Comma>,
}

pub enum Property {
  Name(Ident),
  Member {
    target: Ident,
    dot: Dot,
    member: Ident,
  },
}
pub struct PropMacro {
  pub prop: Property,
  pub comma: Option<Comma>,
  pub lerp_fn: Option<TrackExpr>,
}

impl Parse for PropMacro {
  fn parse(input: ParseStream) -> Result<Self> {
    let prop = if input.peek2(Dot) {
      Property::Member {
        target: input.parse()?,
        dot: input.parse()?,
        member: input.parse()?,
      }
    } else {
      Property::Name(input.parse()?)
    };
    let comma = input.parse()?;
    let lerp_fn = if input.is_empty() {
      None
    } else {
      Some(input.parse::<Expr>()?.into())
    };
    Ok(Self { prop, comma, lerp_fn })
  }
}

impl Parse for MacroSyntax {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut widget: Option<DeclareWidget> = None;
    let mut items = vec![];
    let mut init: Option<Init> = None;
    let mut finally: Option<Finally> = None;
    let mut states: Option<States> = None;
    loop {
      if input.is_empty() {
        break;
      }
      let lk = input.lookahead1();
      if lk.peek(kw::Animate) {
        items.push(Item::Animate(input.parse()?));
      } else if lk.peek(kw::Transition) {
        items.push(Item::Transition(input.parse()?));
      } else if lk.peek(kw::transition) {
        items.push(Item::TransProps(input.parse()?));
      } else if lk.peek(kw::states) {
        let mut t = input.parse::<States>()?;
        if let Some(ot) = states.take() {
          t.states.extend(ot.states);
        }
        states = Some(t);
      } else if lk.peek(kw::init) {
        let e: Init = input.parse::<Init>()?;
        if let Some(init) = init.as_mut() {
          init.block.stmts.extend(e.block.stmts);
        } else {
          init = Some(e)
        }
      } else if lk.peek(kw::finally) {
        let e: Finally = input.parse::<Finally>()?;
        if let Some(finally) = finally.as_mut() {
          finally.block.stmts.extend(e.block.stmts);
        } else {
          finally = Some(e)
        }
      } else if peek_widget(&input) {
        let w: DeclareWidget = input.parse()?;
        if let Some(first) = widget.as_ref() {
          let err = syn::Error::new(
            w.span(),
            format!(
              "Only one root widget can declare, but `{}` already declared.",
              first.ty_path().to_token_stream()
            ),
          );
          return Err(err);
        }
        widget = Some(w);
      } else {
        return Err(lk.error());
      }
    }
    let widget = widget
      .ok_or_else(|| syn::Error::new(input.span(), "must declare a root widget in `widget!`"))?;
    Ok(Self { init, widget, items, states, finally })
  }
}

impl Parse for States {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;

    let states = States {
      _states_token: input.parse()?,
      _brace: braced!(content in input),
      states: {
        let fields: Punctuated<StateField, Comma> = content.parse_terminated(StateField::parse)?;
        fields.into_iter().collect()
      },
    };
    Ok(states)
  }
}

fn ctx_block_parse<Kw: Parse>(
  input: ParseStream,
) -> Result<(Kw, Option<Ident>, Option<FatArrow>, Block)> {
  let kw = input.parse::<Kw>()?;
  let ctx: Option<Ident> = input.parse()?;
  let fat_arrow: Option<FatArrow> = if ctx.is_some() { input.parse()? } else { None };

  Ok((kw, ctx, fat_arrow, input.parse()?))
}

impl Parse for Init {
  fn parse(input: ParseStream) -> Result<Self> {
    ctx_block_parse(input).map(|(_init_token, ctx_name, _fat_arrow, block)| Self {
      _init_token,
      ctx_name,
      _fat_arrow,
      block,
    })
  }
}

impl Parse for Finally {
  fn parse(input: ParseStream) -> Result<Self> {
    ctx_block_parse(input).map(|(_finally_token, ctx_name, _fat_arrow, block)| Self {
      _finally_token,
      ctx_name,
      _fat_arrow,
      block,
    })
  }
}

impl Parse for FieldColon {
  fn parse(input: ParseStream) -> Result<Self> {
    if input.peek(kw::AssignColon) {
      Ok(FieldColon::AssignColon(input.parse()?))
    } else {
      Ok(FieldColon::Colon(input.parse::<Colon>()?))
    }
  }
}

impl Parse for StateField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let member = input.parse::<Ident>()?;
    let (colon_token, expr) = if input.peek(Colon) {
      (Some(input.parse()?), input.parse()?)
    } else {
      (None, parse_quote!(#member))
    };
    Ok(StateField { member, colon_token, expr })
  }
}

impl Parse for Id {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    Ok(Self {
      id: input.parse()?,
      colon: input.parse()?,
      name: input.parse()?,
      tail_comma: input.parse()?,
    })
  }
}

impl Parse for DeclareWidget {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let path: Path = input.parse()?;

    // we not allow an ident as a widget, ambiguous with shorthand field init.
    if input.peek(Paren) {
      let content;
      Ok(DeclareWidget::Call(ConstructCall {
        path,
        paren: syn::parenthesized!(content in input),
        args: content.parse_terminated(Expr::parse)?,
      }))
    } else if input.peek(Brace) {
      let content;
      let brace = syn::braced!(content in input);
      let mut fields = Punctuated::default();
      let mut children = vec![];

      loop {
        if content.is_empty() {
          break;
        }

        if peek_widget(&content) {
          children.push(content.parse()?);
        } else {
          let f: DeclareField = content.parse()?;
          if !children.is_empty() {
            return Err(syn::Error::new(
              f.span(),
              "Field should always declare before children.",
            ));
          }
          fields.push(f);
          if !content.is_empty() {
            content.parse::<Comma>()?;
          }
        }
      }
      check_duplicate_field(&fields)?;

      Ok(DeclareWidget::Literal { ty: path, brace, fields, children })
    } else {
      Ok(DeclareWidget::Path(path))
    }
  }
}

fn peek_widget(input: ParseStream) -> bool {
  (input.peek(Ident) && (input.peek2(Brace) || input.peek2(Colon2) || input.peek2(Paren)))
    || input.peek(Colon2)
    || input.peek2(Colon2)
}

impl Parse for DeclareField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let member: Ident = input.parse()?;
    let mut colon_token = None;
    if input.peek(Colon) {
      colon_token = Some(input.parse()?);
    }
    let expr = if colon_token.is_some() {
      input.parse()?
    } else {
      parse_quote!(#member)
    };

    Ok(DeclareField { member, colon: colon_token, expr })
  }
}

impl Parse for TransProps {
  fn parse(input: ParseStream) -> Result<Self> {
    let transition = input.parse()?;
    let props = if input.peek(Paren) {
      let content;
      parenthesized!(content in input );
      content
        .parse_terminated::<_, Comma>(Expr::parse)?
        .into_iter()
        .collect()
    } else {
      smallvec![input.parse()?]
    };
    let content;
    let brace = braced!(content in input);
    let fields = content.parse_terminated(DeclareField::parse)?;
    Ok(Self { transition, props, brace, fields })
  }
}

impl Parse for DeclareSingle {
  fn parse(input: ParseStream) -> Result<Self> {
    let content;
    let res = Self {
      ty: input.parse()?,
      brace: braced!( content in input),
      fields: content.parse_terminated(DeclareField::parse)?,
    };
    check_duplicate_field(&res.fields)?;
    Ok(res)
  }
}

impl Spanned for DeclareWidget {
  fn span(&self) -> proc_macro2::Span {
    match self {
      DeclareWidget::Literal { ty, brace, .. } => ty.span().join(brace.span).unwrap(),
      DeclareWidget::Path(path) => path.span(),
      DeclareWidget::Call(ConstructCall { path: ty, paren, .. }) => {
        ty.span().join(paren.span).unwrap()
      }
    }
  }
}

impl DeclareWidget {
  pub(crate) fn ty_path(&self) -> &Path {
    match self {
      DeclareWidget::Literal { ty, .. } => ty,
      DeclareWidget::Call(call) => &call.path,
      DeclareWidget::Path(path) => path,
    }
  }
}

impl ToTokens for FieldColon {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      FieldColon::Colon(c) => c.to_tokens(tokens),
      FieldColon::AssignColon(a) => a.to_tokens(tokens),
    }
  }
}

impl ToTokens for DeclareField {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.member.to_tokens(tokens);
    if self.colon.is_some() {
      self.colon.to_tokens(tokens);
      self.expr.to_tokens(tokens);
    }
  }
}

impl ToTokens for ConstructCall {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { path, paren, args } = self;
    path.to_tokens(tokens);
    paren.surround(tokens, |tokens| args.to_tokens(tokens));
  }
}

impl ToTokens for Id {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.id.to_tokens(tokens);
    self.colon.to_tokens(tokens);
    self.name.to_tokens(tokens);
  }
}

pub fn check_duplicate_field(fields: &Punctuated<DeclareField, Comma>) -> syn::Result<()> {
  let mut sets = HashSet::<&Ident, ahash::RandomState>::default();
  for f in fields {
    if !sets.insert(&f.member) {
      return Err(syn::Error::new(
        f.member.span(),
        format!("`{}` declare more than once", f.member).as_str(),
      ));
    }
  }
  Ok(())
}
