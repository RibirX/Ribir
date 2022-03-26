use std::collections::{BTreeMap, HashMap};

use ahash::RandomState;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
  bracketed,
  parse::{Parse, ParseBuffer, ParseStream},
  punctuated::Punctuated,
  spanned::Spanned,
  token::{self, Brace},
  visit_mut::VisitMut,
  Expr, Ident, Path,
};

use crate::{
  declare_func_derive::{ribir_prefix_variable, ReferenceInfo},
  error::DeclareError,
};

use super::{
  child_variable, kw, ribir_variable, sugar_fields::assign_uninit_field, sugar_fields::Id,
  widget_def_variable, widget_gen::WidgetGen, widget_macro::IfGuard, DeclareCtx, FollowOn,
  FollowPart, Follows, Result, SugarFields,
};

pub struct DeclareWidget {
  pub path: Path,
  brace_token: Brace,
  // the name of this widget specified by `id` attr.
  pub named: Option<Id>,
  fields: Vec<DeclareField>,
  sugar_fields: SugarFields,
  children: Vec<Child>,
}

pub enum Child {
  Declare(Box<DeclareWidget>),
  Expr(Box<syn::Expr>),
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
    fn peek2_none(input: ParseBuffer) -> bool { input.parse::<Ident>().is_ok() && input.is_empty() }

    fn is_field(input: ParseStream) -> bool {
      input.peek(Ident)
        && (input.peek2(token::If)
          || input.peek2(token::Colon)
          || input.peek2(token::Comma)
          || peek2_none(input.fork()))
        || input.fork().parse::<SkipNcAttr>().is_ok()
    }

    fn parse_fields(input: ParseStream) -> syn::Result<Punctuated<DeclareField, token::Comma>> {
      let mut punctuated = Punctuated::new();
      while is_field(input) {
        punctuated.push(input.parse()?);
        if input.is_empty() {
          break;
        }
        punctuated.push_punct(input.parse()?);
      }
      Ok(punctuated)
    }

    let content;
    let mut widget = DeclareWidget {
      path: input.parse()?,
      brace_token: syn::braced!(content in input),
      named: None,
      fields: <_>::default(),
      sugar_fields: <_>::default(),
      children: vec![],
    };

    let fields = parse_fields(&content)?;

    fields
      .into_pairs()
      .try_for_each::<_, syn::Result<()>>(|pair| {
        let (f, _) = pair.into_tuple();

        let member = &f.member;
        if syn::parse2::<kw::id>(quote! { #member}).is_ok() {
          let name = Id::from_declare_field(f)?;
          assign_uninit_field!(widget.named, name)?;
        } else if let Some(f) = widget.sugar_fields.assign_field(f)? {
          widget.fields.push(f);
        }
        Ok(())
      })?;

    loop {
      // Expr child should not a `Type` or `Path`, if it's a `Ident`（`Path`), it's
      // ambiguous  with `DeclareChild`, and prefer as `DeclareField`.
      match content.fork().parse() {
        Err(_) if !(content.peek(Ident) && content.peek2(Brace)) => break,
        Ok(Child::Expr(c)) if matches!(*c, Expr::Path(_)) || matches!(*c, Expr::Type(_)) => break,
        _ => {}
      }

      widget.children.push(content.parse()?);
      // Comma follow Child is option.
      let _: Option<token::Comma> = content.parse()?;
    }

    // syntax error hint.
    if !content.is_empty() && is_field(&content) {
      let f: DeclareField = content.parse()?;
      if !widget.children.is_empty() {
        return Err(syn::Error::new(
          f.span(),
          "Field should always declare before children.",
        ));
      }
    }

    Ok(widget)
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

impl DeclareCtx {
  pub fn visit_declare_widget_mut(&mut self, w: &mut DeclareWidget) {
    fn visit_self_only(w: &mut DeclareWidget, ctx: &mut DeclareCtx) {
      ctx.stack_push();
      w.fields
        .iter_mut()
        .for_each(|f| ctx.visit_declare_field_mut(f));

      ctx.visit_sugar_field_mut(&mut w.sugar_fields);
      if let Some(Id { name, .. }) = w.named.as_ref() {
        // named widget followed by attributes or listeners should also mark be followed
        // because it's need capture its state reference to set value.
        let followed_by_attr = w
          .sugar_fields
          .normal_attr_iter()
          .chain(w.sugar_fields.listeners_iter())
          .any(|f| f.follows.is_some());

        if followed_by_attr {
          ctx.add_reference(name.clone(), ReferenceInfo::BeFollowed);
        }
      }

      ctx.stack_pop()
    }
    visit_self_only(w, self);
    w.children.iter_mut().for_each(|c| match c {
      Child::Declare(d) => visit_self_only(d, self),
      Child::Expr(expr) => {
        self.stack_push();
        self.borrow_capture_scope(false).visit_expr_mut(expr);
        self.stack_pop();
        self.take_current_follows();
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

  pub fn visit_sugar_field_mut(&mut self, sugar_field: &mut SugarFields) {
    sugar_field.visit_sugar_field_mut(self);
  }
}

impl DeclareWidget {
  pub fn host_widget_tokens(&self, ctx: &DeclareCtx) -> (Ident, TokenStream) {
    let Self { path: ty, fields, .. } = self;
    let attrs_follow = self
      .sugar_fields
      .normal_attr_iter()
      .any(|f| f.follows.is_some());

    let name = self.widget_identify();
    let gen = WidgetGen { ty, name, fields };

    let mut tokens = gen.gen_widget_tokens(ctx, attrs_follow);
    self.normal_attrs_tokens(&mut tokens);
    self.listeners_tokens(&mut tokens);
    (gen.name.clone(), tokens)
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
            let mut child_tokens = quote! {};
            d.widget_full_tokens(ctx, &mut child_tokens);
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
    compose_tokens.extend(self.sugar_fields.gen_wrap_widget_compose_tokens(&name));

    compose_tokens
  }

  // return this widget tokens and its def name;
  pub fn widget_full_tokens(&self, ctx: &DeclareCtx, tokens: &mut TokenStream) {
    let (name, widget_tokens) = self.host_widget_tokens(ctx);
    tokens.extend(widget_tokens);

    self
      .sugar_fields
      .gen_wrap_widgets_tokens(&name, ctx, |_, wrap_widget| {
        tokens.extend(wrap_widget);
      });

    self.children_tokens(ctx, tokens);
    tokens.extend(self.compose_tokens());
  }

  // return the key-value map of the named widget define tokens.
  pub fn named_objects_def_tokens(
    &self,
    named_defs: &mut HashMap<Ident, TokenStream, RandomState>,
    ctx: &DeclareCtx,
  ) {
    self.traverses_declare().for_each(|w| {
      if w.named.is_some() {
        let (name, def_tokens) = w.host_widget_tokens(ctx);
        named_defs.insert(name.clone(), def_tokens);

        w.sugar_fields
          .gen_wrap_widgets_tokens(&name, ctx, |name, wrap_tokens| {
            named_defs.insert(name, wrap_tokens);
          });
      }
    });
  }

  pub fn normal_attrs_tokens(&self, tokens: &mut TokenStream) {
    let w_name = widget_def_variable(&self.widget_identify());

    self.sugar_fields.normal_attr_iter().for_each(
      |DeclareField {
         expr,
         member,
         follows,
         skip_nc,
         if_guard,
         ..
       }| {
        let method = Ident::new(&format!("with_{}", quote! {#member}), member.span());
        let depends_tokens = follows.as_ref().map(|follows| {
          let upstream = upstream_observable(follows);
          let set_attr = Ident::new(&format!("try_set_{}", quote! {#member}), member.span());
          let get_attr = Ident::new(&format!("get_{}", quote! {#member}), member.span());

          let self_ref = self.widget_identify();
          let value = ribir_variable("v", expr.span());
          let mut assign_value = quote! { #self_ref.silent().#set_attr(#value); };
          if skip_nc.is_some() {
            assign_value = quote! {
              if #self_ref.#get_attr().as_ref() != Some(&#value) {
                #assign_value
              }
            };
          }

          quote! {
            #upstream.subscribe(
              move |_| {
                let #value = #expr;
                #assign_value
              }
            );
          }
        });
        let attr_tokens = quote! {
          #depends_tokens
          let #w_name = #w_name.#method(#expr);
        };
        if let Some(if_guard) = if_guard {
          tokens.extend(quote! {
            let #w_name = #if_guard {
              #attr_tokens
              #w_name
            }  else {
              // insert a empty attr for if-else type compatibility
              #w_name.insert_attr(())
            };
          })
        } else {
          tokens.extend(attr_tokens)
        }
      },
    )
  }

  pub fn listeners_tokens(&self, tokens: &mut TokenStream) {
    let name = widget_def_variable(&self.widget_identify());

    let (guards, without_guards) = self
      .sugar_fields
      .listeners_iter()
      .partition::<Vec<_>, _>(|f| f.if_guard.is_some());
    guards
      .iter()
      .for_each(|DeclareField { expr, member, if_guard, .. }| {
        let if_guard = if_guard.as_ref().unwrap();
        tokens.extend(quote! {
          let #name =  #if_guard {
            #name.#member(#expr)
          } else {
            // insert a empty attr for if-else type compatibility
            #name.insert_attr(())
          };
        });
      });

    if !without_guards.is_empty() {
      let attrs = without_guards
        .iter()
        .map(|DeclareField { expr, member, .. }| {
          quote! {
            .#member(#expr)
          }
        });

      tokens.extend(quote! { let #name = #name #(#attrs)*; });
    }
  }

  /// Return a iterator of all syntax fields, include attributes and wrap
  /// widget.
  pub fn all_syntax_fields(&self) -> impl Iterator<Item = &DeclareField> {
    self
      .fields
      .iter()
      .chain(self.sugar_fields.normal_attr_iter())
      .chain(self.sugar_fields.listeners_iter())
      .chain(self.sugar_fields.widget_wrap_field_iter())
  }

  pub fn before_generate_check(&self, ctx: &DeclareCtx) -> Result<()> {
    self.traverses_declare().try_for_each(|w| {
      if w.named.is_some() {
        w.unnecessary_skip_nc_check()?;
        w.wrap_widget_if_guard_check(ctx)?;
      }
      w.sugar_fields.key_follow_check()
    })
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
        w.sugar_fields
          .collect_wrap_widget_follows(&ref_name, &mut follows);

        let w_follows: Follows = w
          .fields
          .iter()
          .filter_map(FollowPart::from_widget_field)
          .chain(
            w.sugar_fields
              .normal_attr_iter()
              .chain(w.sugar_fields.listeners_iter())
              .filter_map(FollowPart::from_widget_field),
          )
          .collect();
        if !w_follows.is_empty() {
          follows.insert(ref_name, w_follows);
        }
      });

    follows
  }

  fn unnecessary_skip_nc_check(&self) -> Result<()> {
    debug_assert!(self.named.is_some());
    fn unnecessary_skip_nc(
      DeclareField { skip_nc, follows: depends_on, .. }: &DeclareField,
    ) -> Result<()> {
      match (depends_on, skip_nc) {
        (None, Some(attr)) => Err(DeclareError::UnnecessarySkipNc(attr.span().unwrap())),
        _ => Ok(()),
      }
    }

    // normal widget
    self
      .fields
      .iter()
      .chain(self.sugar_fields.normal_attr_iter())
      .try_for_each(unnecessary_skip_nc)?;

    self
      .sugar_fields
      .widget_wrap_field_iter()
      .try_for_each(unnecessary_skip_nc)
  }

  fn wrap_widget_if_guard_check(&self, ctx: &DeclareCtx) -> Result<()> {
    debug_assert!(self.named.is_some());

    self
      .sugar_fields
      .widget_wrap_field_iter()
      .filter(|f| f.if_guard.is_some())
      .try_for_each(|f| {
        let w_ref = self.widget_identify();
        let wrap_name = ribir_prefix_variable(&f.member, &w_ref.to_string());

        if ctx.be_followed(&wrap_name) {
          let if_guard_span = f.if_guard.as_ref().unwrap().span().unwrap();
          let mut use_spans = vec![];
          self.traverses_declare().for_each(|w| {
            w.all_syntax_fields()
              .filter_map(|f| f.follows.as_ref())
              .flat_map(|follows| follows.iter())
              .filter(|f| f.widget == wrap_name)
              .for_each(|f| use_spans.extend(f.spans.iter().map(|s| s.unwrap())))
          });

          let host_span = w_ref.span().unwrap();
          let wrap_span = wrap_name.span().unwrap();
          return Err(DeclareError::DependOnWrapWidgetWithIfGuard {
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
      Child::Declare(w) => Some(&**w),
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
    if input.peek(Ident) && input.peek2(Brace) {
      Ok(Child::Declare(input.parse()?))
    } else {
      Ok(Child::Expr(input.parse()?))
    }
  }
}
