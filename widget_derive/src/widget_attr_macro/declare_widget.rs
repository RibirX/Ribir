use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::collections::BTreeMap;
use syn::{
  bracketed,
  parse::{Parse, ParseStream},
  spanned::Spanned,
  token::{self, Brace},
  visit_mut::VisitMut,
  Expr, Ident, Path,
};
mod widget_gen;
use crate::{
  error::{DeclareError, DeclareWarning},
  widget_attr_macro::ribir_prefix_variable,
};
mod builtin_fields;
pub use builtin_fields::*;
use widget_gen::WidgetGen;

use super::{
  child_variable, kw, ribir_variable, widget_def_variable,
  widget_macro::{is_expr_keyword, IfGuard, UsedNameInfo, EXPR_FIELD, EXPR_WIDGET},
  DeclareCtx, DependIn, DependPart, Depends, Id, Result,
};

#[derive(Debug)]
pub struct DeclareWidget {
  pub path: Path,
  brace_token: Brace,
  // the name of this widget specified by `id` attr.
  pub named: Option<Id>,
  fields: Vec<DeclareField>,
  builtin: BuiltinFieldWidgets,
  pub children: Vec<Box<DeclareWidget>>,
}

#[derive(Clone, Debug)]
pub struct DeclareField {
  pub skip_nc: Option<SkipNcAttr>,
  pub member: Ident,
  pub if_guard: Option<IfGuard>,
  pub colon_token: Option<token::Colon>,
  pub expr: Expr,
  pub used_name_info: UsedNameInfo,
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

impl Spanned for DeclareWidget {
  fn span(&self) -> Span { self.path.span().join(self.brace_token.span).unwrap() }
}

impl Parse for DeclareWidget {
  fn parse(input: ParseStream) -> syn::Result<Self> {
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

      if content.peek(Ident) && content.peek2(token::Brace) {
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
      used_name_info: <_>::default(),
    })
  }
}

impl DeclareField {
  pub fn depend_parts(&self) -> impl Iterator<Item = DependPart> + '_ {
    self.used_name_info.depend_parts(DependIn::Field(self))
  }
}

pub fn try_parse_skip_nc(input: ParseStream) -> syn::Result<Option<SkipNcAttr>> {
  if input.peek(token::Pound) {
    Ok(Some(input.parse()?))
  } else {
    Ok(None)
  }
}

impl DeclareCtx {
  pub fn visit_declare_widget_mut(&mut self, w: &mut DeclareWidget) {
    let mut ctx = self.stack_push();
    w.fields
      .iter_mut()
      .for_each(|f| ctx.visit_declare_field_mut(f));

    ctx.visit_builtin_field_widgets(&mut w.builtin);

    w.children
      .iter_mut()
      .for_each(|c| ctx.visit_declare_widget_mut(c))
  }

  pub fn visit_declare_field_mut(&mut self, f: &mut DeclareField) {
    self.visit_ident_mut(&mut f.member);
    if let Some(if_guard) = f.if_guard.as_mut() {
      self.visit_if_guard_mut(if_guard);
    }
    self.visit_expr_mut(&mut f.expr);

    f.used_name_info = self.take_current_used_info();
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

  pub fn compose_tokens(&self, ctx: &DeclareCtx) -> TokenStream {
    let mut compose_tokens = quote! {};
    let name = &self.widget_identify();
    let def_name = widget_def_variable(name);
    let children = &self.children;

    if !children.is_empty() {
      // Must be MultiChild if there are multi child. Give this hint for better
      // compile error if wrong size child declared.
      let hint = (self.children.len() > 1).then(|| quote! {: MultiChildWidget<_>});

      let children = children.iter().enumerate().map(|(idx, c)| {
        let child_name = if c.named.is_none() {
          widget_def_variable(&child_variable(c, idx))
        } else {
          let child_widget_name = widget_def_variable(&c.widget_identify());
          let child_compose = c.compose_tokens(ctx);
          compose_tokens.extend(child_compose);
          child_widget_name
        };

        if c
          .builtin
          .finally_is_expr_widget()
          .unwrap_or_else(|| c.is_host_expr_widget())
        {
          quote_spanned! { c.span() => .have_expr_child(#child_name)  }
        } else {
          quote_spanned! { c.span() => .have_child(#child_name) }
        }
      });
      let compose_children = quote! { let #def_name #hint = #def_name #(#children)*; };
      compose_tokens.extend(compose_children);
    }
    compose_tokens.extend(self.builtin.compose_tokens(self));
    compose_tokens
  }

  pub fn gen_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    // only generate anonyms widget tokens, named widget pre-generate before.
    if self.named.is_none() {
      let (name, host_widget) = self.host_widget_tokens(ctx);
      tokens.extend(host_widget);
      self
        .builtin
        .widget_tokens_iter(name, ctx)
        .for_each(|(_, builtin_widget)| {
          tokens.extend(builtin_widget);
        })
    }

    self
      .children
      .iter()
      .enumerate()
      .filter(|(_, c)| c.named.is_none())
      .for_each(|(idx, c)| {
        let c_name = widget_def_variable(&child_variable(c, idx));
        let child_widget_name = widget_def_variable(&c.widget_identify());
        tokens.extend(quote_spanned! { c.path.span() =>  let #c_name = });
        c.brace_token.surround(tokens, |tokens| {
          c.gen_tokens(ctx, tokens);
          child_widget_name.to_tokens(tokens);
        });
        token::Semi::default().to_tokens(tokens);
      });

    tokens.extend(self.compose_tokens(ctx));
  }

  // return the key-value map of the named widget define tokens.
  pub fn named_objects_def_tokens_iter<'a>(
    &'a self,
    ctx: &'a DeclareCtx,
  ) -> impl Iterator<Item = (Ident, TokenStream)> + 'a {
    self
      .traverses_widget()
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
    self.traverses_widget().try_for_each(|w| {
      if w.named.is_some() {
        w.builtin_field_if_guard_check(ctx)?;
      }
      if is_expr_keyword(&w.path) {
        if w.fields.len() != 1 || w.fields[0].member != EXPR_FIELD {
          let spans = w.fields.iter().map(|f| f.member.span().unwrap()).collect();
          return Err(DeclareError::ExprWidgetInvalidField(spans));
        }
        if let Some(guard) = w.fields[0].if_guard.as_ref() {
          return Err(DeclareError::UnsupportedIfGuard {
            name: format!("field {EXPR_FIELD} of  {EXPR_WIDGET}"),
            span: guard.span().unwrap(),
          });
        }
      }

      w.builtin.key_follow_check()
    })
  }

  pub fn warnings(&self) -> impl Iterator<Item = DeclareWarning> + '_ {
    self
      .fields
      .iter()
      .chain(self.builtin.all_builtin_fields())
      .filter(|f| self.named.is_none() || !f.used_name_info.use_or_capture_any_name())
      .filter_map(|f| {
        f.skip_nc
          .as_ref()
          .map(|attr| DeclareWarning::NeedlessSkipNc(attr.span().unwrap()))
      })
      .chain(self.children.iter().flat_map(|c| {
        let iter: Box<dyn Iterator<Item = DeclareWarning>> = Box::new(c.warnings());
        iter
      }))
  }

  /// return follow relationship of the named widgets,it is a key-value map,
  /// schema like
  /// ``` ascii
  /// {
  ///   widget_name: [field, {depended_widget: [position]}]
  /// }
  /// ```
  pub fn analyze_object_follows(&self) -> BTreeMap<Ident, Depends> {
    let mut follows: BTreeMap<Ident, Depends> = BTreeMap::new();
    self
      .traverses_widget()
      .filter(|w| w.named.is_some())
      .for_each(|w| {
        let ref_name = w.widget_identify();
        w.builtin
          .collect_builtin_widget_follows(&ref_name, &mut follows);

        let w_follows: Depends = w.fields.iter().flat_map(|f| f.depend_parts()).collect();

        if !w_follows.is_empty() {
          follows.insert(ref_name, w_follows);
        }
      });

    follows
  }

  pub(crate) fn is_host_expr_widget(&self) -> bool {
    // if `ExprWidget` track nothing, will not as a `ExprWidget`, but use its
    // directly return value.
    is_expr_keyword(&self.path)
      && self
        .fields
        .iter()
        .any(|f| f.used_name_info.used_names.is_some())
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

        if ctx.is_used(&wrap_name) {
          let if_guard_span = f.if_guard.as_ref().unwrap().span().unwrap();
          let mut use_spans = vec![];
          self.traverses_widget().for_each(|w| {
            w.builtin
              .all_builtin_fields()
              .filter_map(|f| f.used_name_info.used_names.as_ref())
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
      .traverses_widget()
      .filter_map(|w| w.named.as_ref().map(|id| &id.name))
  }

  pub fn traverses_widget(&self) -> impl Iterator<Item = &DeclareWidget> {
    let children: Box<dyn Iterator<Item = &DeclareWidget>> =
      Box::new(self.children.iter().flat_map(|w| w.traverses_widget()));

    std::iter::once(self).chain(children)
  }

  pub fn widget_identify(&self) -> Ident {
    match &self.named {
      Some(Id { name, .. }) => name.clone(),
      _ => ribir_variable("ribir", self.path.span()),
    }
  }
}

pub fn upstream_by_used_widgets<'a>(
  used_widgets: impl Iterator<Item = &'a Ident> + Clone,
) -> TokenStream {
  let upstream = used_widgets.clone().map(|w| {
    let w = widget_def_variable(w);
    quote_spanned! { w.span() =>  #w.change_stream() }
  });
  if used_widgets.count() > 1 {
    quote! {  observable::from_iter([#(#upstream),*]).merge_all(usize::MAX) }
  } else {
    quote! { #(#upstream)* }
  }
}
