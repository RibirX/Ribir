use proc_macro2::{Delimiter, Group, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use std::collections::BTreeMap;
use syn::{
  bracketed,
  buffer::Cursor,
  parse::{discouraged::Speculative, Parse, ParseStream},
  spanned::Spanned,
  token::{self, Brace},
  visit_mut::VisitMut,
  Expr, Ident, Path,
};
mod widget_gen;
use crate::{
  error::{DeclareError, DeclareWarning},
  widget_attr_macro::{ribir_prefix_variable, DECLARE_WRAP_MACRO},
};
mod builtin_fields;
pub use builtin_fields::*;
use widget_gen::WidgetGen;

use super::{
  child_variable, kw, ribir_variable, widget_def_variable, widget_macro::IfGuard, DeclareCtx,
  FollowOn, FollowPart, Follows, Id, Result,
};

#[derive(Debug)]
pub struct DeclareWidget {
  declare_token: Option<kw::declare>,
  pub path: Path,
  brace_token: Brace,
  // the name of this widget specified by `id` attr.
  pub named: Option<Id>,
  fields: Vec<DeclareField>,
  builtin: BuiltinFieldWidgets,
  pub children: Vec<Child>,
}

#[derive(Debug)]
pub enum Child {
  Declare(Box<DeclareWidget>),
  Expr(ExprChild),
}

#[derive(Debug)]
pub struct ExprChild {
  _expr_child: kw::ExprChild,
  expr: syn::ExprBlock,
}
#[derive(Clone, Debug)]
pub struct DeclareField {
  pub skip_nc: Option<SkipNcAttr>,
  pub member: Ident,
  pub if_guard: Option<IfGuard>,
  pub colon_token: Option<token::Colon>,
  pub expr: Expr,
  pub follows: Option<Vec<FollowOn>>,
}

#[derive(Clone, Debug)]
pub struct SkipNcAttr {
  pound_token: token::Pound,
  bracket_token: token::Bracket,
  skip_nc_meta: kw::skip_nc,
}

macro_rules! assign_uninit_field {
  ($self: ident.$name: ident, $field: ident) => {
    assign_uninit_field!($self.$name, $field, $name)
  };
  ($left: expr, $right: ident, $name: ident) => {
    if $left.is_none() {
      $left = Some($right);
      Ok(())
    } else {
      Err(syn::Error::new(
        $right.span(),
        format!("`{}` declare more than once", stringify!($name)).as_str(),
      ))
    }
  };
}

pub(crate) use assign_uninit_field;

impl ToTokens for SkipNcAttr {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.pound_token.to_tokens(tokens);
    self.bracket_token.surround(tokens, |tokens| {
      self.skip_nc_meta.to_tokens(tokens);
    })
  }
}

impl ToTokens for DeclareField {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.member.to_tokens(tokens);
    self.colon_token.to_tokens(tokens);
    let expr = &self.expr;
    if let Some(if_guard) = self.if_guard.as_ref() {
      tokens.extend(quote! {
        #if_guard {
          #expr
        } else {
          <_>::default()
        }
      })
    } else if self.colon_token.is_some() {
      expr.to_tokens(tokens)
    }
  }
}

impl ToTokens for ExprChild {
  fn to_tokens(&self, tokens: &mut TokenStream) { self.expr.to_tokens(tokens) }
}

impl Spanned for DeclareWidget {
  fn span(&self) -> Span { self.path.span().join(self.brace_token.span).unwrap() }
}

impl Parse for DeclareWidget {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let _declare_token = input.parse()?;
    let path = input.parse()?;
    let content;
    let brace_token = syn::braced!(content in input);
    let mut named: Option<Id> = None;
    let mut fields = vec![];
    let mut builtin = BuiltinFieldWidgets::default();
    let mut children = vec![];
    loop {
      if content.is_empty() {
        break;
      }

      if (content.peek(Ident) && content.peek2(token::Brace))
        || (content.peek(kw::declare) && content.peek2(Ident) && content.peek3(token::Brace))
      {
        children.push(content.parse()?);
      } else {
        let is_id = content.peek(kw::id);
        let f: DeclareField = content.parse()?;
        if !children.is_empty() {
          return Err(syn::Error::new(
            f.span(),
            "Field should always declare before children.",
          ));
        }

        if is_id {
          let id = Id::from_declare_field(f)?;
          assign_uninit_field!(named, id, id)?;
        } else if let Some(ty) = FIELD_WIDGET_TYPE.get(f.member.to_string().as_str()) {
          builtin.assign_builtin_field(ty, f)?;
        } else {
          fields.push(f);
        }

        if !content.is_empty() {
          content.parse::<token::Comma>()?;
        }
      }
    }

    Ok(DeclareWidget {
      declare_token: _declare_token,
      path,
      brace_token,
      named,
      fields,
      builtin,
      children,
    })
  }
}

impl Parse for SkipNcAttr {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let pound_token = input.parse()?;
    let content;
    let bracket_token = bracketed!(content in input);
    Ok(Self {
      pound_token,
      bracket_token,
      skip_nc_meta: content.parse()?,
    })
  }
}

impl Parse for DeclareField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let skip_nc = try_parse_skip_nc(input)?;
    let member: Ident = input.parse()?;
    let if_guard = if input.peek(token::If) {
      Some(input.parse()?)
    } else {
      None
    };
    let colon_token: Option<_> = if if_guard.is_some() {
      Some(input.parse()?)
    } else {
      input.parse()?
    };

    let expr = if colon_token.is_some() {
      input.parse()?
    } else {
      Expr::Path(syn::ExprPath {
        attrs: Vec::new(),
        qself: None,
        path: Path::from(member.clone()),
      })
    };

    Ok(DeclareField {
      skip_nc,
      member,
      if_guard,
      colon_token,
      expr,
      follows: None,
    })
  }
}

pub fn try_parse_skip_nc(input: ParseStream) -> syn::Result<Option<SkipNcAttr>> {
  if input.peek(token::Pound) {
    Ok(Some(input.parse()?))
  } else {
    Ok(None)
  }
}

impl Parse for ExprChild {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let _expr_child = input.parse()?;
    let wrap_fork = input.fork();
    let expr = if let Some(tokens) =
      wrap_fork.step(|step_cursor| Ok(macro_wrap_declare_keyword(*step_cursor)))?
    {
      input.advance_to(&wrap_fork);
      syn::parse2(tokens.into_iter().collect())?
    } else {
      input.parse()?
    };

    Ok(ExprChild { _expr_child, expr })
  }
}

impl DeclareCtx {
  pub fn visit_declare_widget_mut(&mut self, w: &mut DeclareWidget) {
    let mut ctx = self.stack_push();
    w.fields
      .iter_mut()
      .for_each(|f| ctx.visit_declare_field_mut(f));

    ctx.visit_builtin_field_widgets(&mut w.builtin);

    w.children.iter_mut().for_each(|c| match c {
      Child::Declare(d) => ctx.visit_declare_widget_mut(d),
      Child::Expr(expr) => {
        let mut ctx = ctx.stack_push();
        ctx
          .borrow_capture_scope(false)
          .visit_expr_block_mut(&mut expr.expr);
        ctx.take_current_follows();
      }
    })
  }

  pub fn visit_declare_field_mut(&mut self, f: &mut DeclareField) {
    self.visit_ident_mut(&mut f.member);
    if let Some(if_guard) = f.if_guard.as_mut() {
      self
        .borrow_capture_scope(false)
        .visit_expr_mut(&mut if_guard.cond);
    }
    self.visit_expr_mut(&mut f.expr);

    f.follows = self.take_current_follows();
  }

  pub fn visit_builtin_field_widgets(&mut self, builtin: &mut BuiltinFieldWidgets) {
    builtin.visit_builtin_fields_mut(self);
  }
}

impl DeclareWidget {
  pub fn host_widget_tokens(&self, ctx: &DeclareCtx) -> (Ident, TokenStream) {
    let Self { path: ty, fields, .. } = self;

    let name = self.widget_identify();
    let gen = WidgetGen { ty, name, fields };
    let tokens = gen.gen_widget_tokens(ctx);
    (gen.name, tokens)
  }

  pub fn children_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    self
      .children
      .iter()
      .enumerate()
      .for_each(|(idx, c)| match c {
        Child::Declare(d) => {
          if d.named.is_none() {
            let child_widget_name = widget_def_variable(&d.widget_identify());
            let c_def_name = widget_def_variable(&child_variable(c, idx));
            let child_tokens = d.widget_full_tokens(ctx);
            tokens.extend(quote! { let #c_def_name = { #child_tokens  #child_widget_name }; });
          } else {
            tokens.extend(d.compose_tokens());
          }
        }
        Child::Expr(expr) => {
          let c_name = widget_def_variable(&child_variable(c, idx));
          tokens.extend(quote! { let #c_name = #expr; });
        }
      });
  }

  pub fn compose_tokens(&self) -> TokenStream {
    let mut compose_tokens = quote! {};
    let name = &self.widget_identify();
    let def_name = widget_def_variable(name);
    if !self.children.is_empty() {
      // Must be MultiChild if there are multi child. Give this hint for better
      // compile error if wrong size child declared.
      let hint = (self.children.len() > 1).then(|| quote! {: MultiChild<_>});

      let children = self.children.iter().enumerate().map(|(idx, c)| {
        let c_name = match c {
          Child::Declare(d) if d.named.is_some() => d.widget_identify(),
          _ => child_variable(c, idx),
        };
        let c_def_name = widget_def_variable(&c_name);
        quote! { .have_child(#c_def_name) }
      });
      compose_tokens.extend(quote! { let #def_name #hint = #def_name #(#children)*; });
    }
    compose_tokens.extend(self.builtin.compose_tokens(name));

    compose_tokens
  }

  // return this widget tokens and its def name;
  pub fn widget_full_tokens(&self, ctx: &DeclareCtx) -> TokenStream {
    let (name, mut tokens) = self.host_widget_tokens(ctx);

    self
      .builtin
      .widget_tokens_iter(name, ctx)
      .for_each(|(_, wrap_widget)| {
        tokens.extend(wrap_widget);
      });

    self.children_tokens(ctx, &mut tokens);
    tokens.extend(self.compose_tokens());
    tokens
  }

  // return the key-value map of the named widget define tokens.
  pub fn named_objects_def_tokens_iter<'a>(
    &'a self,
    ctx: &'a DeclareCtx,
  ) -> impl Iterator<Item = (Ident, TokenStream)> + 'a {
    self
      .traverses_declare()
      .filter_map(|w| {
        w.named.as_ref().map(|_| {
          let host = w.host_widget_tokens(ctx);
          let builtin = w.builtin.widget_tokens_iter(host.0.clone(), ctx);
          std::iter::once(host).chain(builtin)
        })
      })
      .flatten()
  }

  pub fn before_generate_check(&self, ctx: &DeclareCtx) -> Result<()> {
    self.traverses_declare().try_for_each(|w| {
      if w.named.is_some() {
        w.builtin_field_if_guard_check(ctx)?;
      }
      w.builtin.key_follow_check()
    })
  }

  pub fn warnings(&self) -> impl Iterator<Item = DeclareWarning> + '_ {
    self
      .fields
      .iter()
      .chain(self.builtin.all_builtin_fields())
      .filter(|f| self.named.is_none() || f.follows.is_none())
      .filter_map(|f| {
        f.skip_nc
          .as_ref()
          .map(|attr| DeclareWarning::NeedlessSkipNc(attr.span().unwrap()))
      })
      .chain(self.children.iter().flat_map(Child::warnings))
  }

  /// return follow relationship of the named widgets,it is a key-value map,
  /// schema like
  /// ``` ascii
  /// {
  ///   widget_name: [field, {depended_widget: [position]}]
  /// }
  /// ```
  pub fn analyze_object_follows(&self) -> BTreeMap<Ident, Follows> {
    let mut follows: BTreeMap<Ident, Follows> = BTreeMap::new();
    self
      .traverses_declare()
      .filter(|w| w.named.is_some())
      .for_each(|w| {
        let ref_name = w.widget_identify();
        w.builtin
          .collect_wrap_widget_follows(&ref_name, &mut follows);

        let w_follows: Follows = w
          .fields
          .iter()
          .filter_map(FollowPart::from_widget_field)
          .collect();
        if !w_follows.is_empty() {
          follows.insert(ref_name, w_follows);
        }
      });

    follows
  }

  fn builtin_field_if_guard_check(&self, ctx: &DeclareCtx) -> Result<()> {
    debug_assert!(self.named.is_some());

    self
      .builtin
      .all_builtin_fields()
      .filter(|f| f.if_guard.is_some())
      .try_for_each(|f| {
        let w_ref = self.widget_identify();
        let wrap_name = ribir_prefix_variable(&f.member, &w_ref.to_string());

        if ctx.be_followed(&wrap_name) {
          let if_guard_span = f.if_guard.as_ref().unwrap().span().unwrap();
          let mut use_spans = vec![];
          self.traverses_declare().for_each(|w| {
            w.builtin
              .all_builtin_fields()
              .filter_map(|f| f.follows.as_ref())
              .flat_map(|follows| follows.iter())
              .filter(|f| f.widget == wrap_name)
              .for_each(|f| use_spans.extend(f.spans.iter().map(|s| s.unwrap())))
          });

          let host_span = w_ref.span().unwrap();
          let wrap_span = wrap_name.span().unwrap();
          return Err(DeclareError::DependOBuiltinFieldWithIfGuard {
            wrap_def_spans: [host_span, wrap_span, if_guard_span],
            use_spans,
            wrap_name,
          });
        }
        Ok(())
      })
  }

  pub fn object_names_iter(&self) -> impl Iterator<Item = &Ident> {
    self
      .traverses_declare()
      .filter_map(|w| w.named.as_ref().map(|id| &id.name))
  }

  /// pre-order traversals declare widget, this will skip the expression child.
  pub fn traverses_declare(&self) -> impl Iterator<Item = &DeclareWidget> {
    let children = self.children.iter().filter_map(|c| match c {
      Child::Declare(w) => Some(w),
      Child::Expr(_) => None,
    });
    let children: Box<dyn Iterator<Item = &DeclareWidget>> =
      Box::new(children.flat_map(|w| w.traverses_declare()));

    std::iter::once(self).chain(children)
  }

  pub fn widget_identify(&self) -> Ident {
    match &self.named {
      Some(Id { name, .. }) => name.clone(),
      _ => ribir_variable("ribir", self.path.span()),
    }
  }
}

pub fn upstream_observable(depends_on: &[FollowOn]) -> TokenStream {
  let upstream = depends_on.iter().map(|fo| {
    let depend_w = &fo.widget;
    quote! { #depend_w.change_stream() }
  });

  if depends_on.len() > 1 {
    quote! {  observable::from_iter([#(#upstream),*]).merge_all(usize::MAX) }
  } else {
    quote! { #(#upstream)* }
  }
}

impl Spanned for Child {
  fn span(&self) -> Span {
    match self {
      Child::Declare(d) => d.span(),
      Child::Expr(e) => e.span(),
    }
  }
}

impl Parse for Child {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    if input.peek(kw::ExprChild) {
      Ok(Child::Expr(input.parse()?))
    } else {
      Ok(Child::Declare(input.parse()?))
    }
  }
}

/// Wrap `declare Row {...}` with macro `ribir_declare_ಠ_ಠ!`, let our syntax
/// as a valid rust expression,  so we can use rust syntax to parse and
/// needn't reimplemented, and easy to interop with rust syntax.
///
/// return new tokens if do any wrap else
fn macro_wrap_declare_keyword(mut cursor: Cursor) -> (Option<Vec<TokenTree>>, Cursor) {
  fn sub_token_stream(mut begin: Cursor, end: Option<Cursor>, tts: &mut Vec<TokenTree>) {
    while Some(begin) != end {
      match begin.token_tree() {
        Some((tt, rest)) => {
          tts.push(tt);
          begin = rest;
        }
        None => break,
      }
    }
  }

  fn group_inner_wrap(cursor: Cursor, delim: Delimiter) -> Option<(Group, Cursor)> {
    cursor
      .group(delim)
      .and_then(|(group_cursor, span, cursor)| {
        macro_wrap_declare_keyword(group_cursor).0.map(|tts| {
          let mut group = Group::new(delim, tts.into_iter().collect());
          group.set_span(span);
          (group, cursor)
        })
      })
  }

  let mut tts = vec![];
  let mut stream_cursor = cursor;
  loop {
    if let Some((n, c)) = cursor.ident() {
      if n == "widget" {
        let widget_macro = c
          .punct()
          .filter(|(p, _)| p.as_char() == '!')
          .and_then(|(_, c)| {
            c.group(Delimiter::Brace)
              .or_else(|| c.group(Delimiter::Parenthesis))
              .or_else(|| c.group(Delimiter::Bracket))
          });
        if let Some((_, _, c)) = widget_macro {
          // skip inner widget! macro, wrap wait itself parse.
          cursor = c;
          continue;
        }
      } else if n == "declare" {
        let declare_group =
          c.ident()
            .map(|(name, c)| (n, name, c))
            .and_then(|(declare, name, c)| {
              c.group(Delimiter::Brace)
                .map(|(body_cursor, span, cursor)| (declare, name, body_cursor, span, cursor))
            });
        if let Some((declare, name, body_cursor, body_span, c)) = declare_group {
          sub_token_stream(stream_cursor, Some(cursor), &mut tts);
          let body = macro_wrap_declare_keyword(body_cursor).0.map_or_else(
            || body_cursor.token_stream(),
            |tokens| tokens.into_iter().collect(),
          );

          tts.push(TokenTree::Ident(Ident::new(
            DECLARE_WRAP_MACRO,
            declare.span(),
          )));
          let mut bang = Punct::new('!', Spacing::Alone);
          bang.set_span(declare.span());
          tts.push(TokenTree::Punct(bang));

          let mut declare_group = Group::new(Delimiter::Brace, body);
          declare_group.set_span(body_span);
          let mut macro_group =
            Group::new(Delimiter::Brace, quote! { #declare #name #declare_group});
          macro_group.set_span(declare.span());
          tts.push(TokenTree::Group(macro_group));

          cursor = c;
          stream_cursor = c;
          continue;
        }
      }
      cursor = c;
    } else if let Some((group, c)) = group_inner_wrap(cursor, Delimiter::Brace)
      .or_else(|| group_inner_wrap(cursor, Delimiter::Bracket))
      .or_else(|| group_inner_wrap(cursor, Delimiter::Parenthesis))
    {
      sub_token_stream(stream_cursor, Some(cursor), &mut tts);
      tts.push(TokenTree::Group(group));
      cursor = c;
      stream_cursor = c;
    } else if let Some((_, c)) = cursor.token_tree() {
      cursor = c;
    } else {
      break;
    }
  }

  let tts = (!tts.is_empty()).then(|| {
    sub_token_stream(stream_cursor, None, &mut tts);
    tts.into_iter().collect()
  });
  (tts, cursor)
}

impl Child {
  fn warnings(&self) -> Box<dyn Iterator<Item = DeclareWarning> + '_> {
    match self {
      Child::Declare(d) => {
        let iter = d
          .declare_token
          .as_ref()
          .map(|d| DeclareWarning::NeedlessDeclare(d.span().unwrap()))
          .into_iter()
          .chain(d.warnings());
        Box::new(iter)
      }
      Child::Expr(_) => Box::new(std::iter::empty()),
    }
  }
}
