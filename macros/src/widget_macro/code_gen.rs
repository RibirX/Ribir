use crate::{
  error::{DeclareError, DeclareWarning},
  widget_macro::WIDGET_OF_BUILTIN_FIELD,
};
use ahash::RandomState;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use smallvec::{smallvec, SmallVec};
use std::collections::{BTreeMap, HashMap, HashSet};
use syn::{
  parse_macro_input, parse_quote,
  spanned::Spanned,
  token::{Brace, Dot, Paren, Semi},
  visit_mut::VisitMut,
  Expr, Ident,
};

use super::{
  builtin_var_name, ctx_ident,
  desugar::{
    ComposeItem, DeclareObj, Field, FieldValue, FinallyBlock, FinallyStmt, InitStmts, NamedObj,
    NamedObjMap, WidgetNode,
  },
  guard_vec_ident,
  parser::{PropMacro, Property, StateField, States},
  Desugared, ObjectUsed, ObjectUsedPath, UsedPart, UsedType, VisitCtx, WIDGETS,
};

pub(crate) fn gen_prop_macro(
  input: proc_macro::TokenStream,
  ctx: &mut VisitCtx,
) -> proc_macro::TokenStream {
  let mut prop_macro = parse_macro_input! { input as PropMacro };
  let PropMacro { prop, lerp_fn, .. } = &mut prop_macro;

  let name = match &prop {
    Property::Name(name) => name,
    Property::Member { target, .. } => target,
  };

  if ctx.find_named_obj(name).is_none() {
    let mut tokens = quote!();
    DeclareError::PropInvalidTarget(name.span()).to_compile_error(&mut tokens);
    return tokens.into();
  };

  ctx.new_scope_visit(
    |ctx| match prop {
      Property::Name(name) => {
        if let Some(name) = ctx.find_named_obj(name).cloned() {
          ctx.add_used_widget(name, None, UsedType::USED)
        }
      }
      Property::Member { target, member, .. } => {
        if let Some(builtin_ty) = WIDGET_OF_BUILTIN_FIELD.get(member.to_string().as_str()) {
          let span = target.span().join(member.span()).unwrap();
          if let Some(name) = ctx.visit_builtin_name_mut(target, span, builtin_ty) {
            *target = name;
          }
        } else if let Some(name) = ctx.find_named_obj(target).cloned() {
          ctx.add_used_widget(name, None, UsedType::USED)
        }
      }
    },
    |scope| {
      scope
        .iter_mut()
        .for_each(|(_, info)| info.used_type.remove(UsedType::SUBSCRIBE))
    },
  );
  if let Some(lerp_fn) = lerp_fn {
    ctx.visit_track_expr_mut(lerp_fn);
  }

  prop_macro.to_token_stream().into()
}

pub(crate) fn gen_move_to_widget_macro(input: &TokenStream, ctx: &mut VisitCtx) -> TokenStream {
  let mut expr: Expr = parse_quote!(#input);
  ctx.visit_expr_mut(&mut expr);
  ctx.has_guards_data = true;
  let guards = guard_vec_ident();
  quote_spanned!(expr.span() => #guards.push(AnonymousData::new(Box::new(#expr))))
}

impl Desugared {
  pub fn gen_tokens(&self, tokens: &mut TokenStream, ctx: &VisitCtx) {
    let Self {
      init,
      named_objs,
      widget,
      states,
      finally,
      errors,
      warnings,
      ..
    } = &self;

    if !errors.is_empty() {
      Brace::default().surround(tokens, |tokens| {
        errors.iter().for_each(|err| err.to_compile_error(tokens));
        quote! { Void.into_widget() }.to_tokens(tokens);
      });

      return;
    }
    if !ctx.visit_error_occur {
      warnings.iter().for_each(|w| w.emit_warning());
    }

    let sorted_named_objs = self.order_named_objs();
    Brace::default().surround(tokens, |tokens| {
      Brace::default().surround(tokens, |tokens| {
        quote!(#![allow(unused_mut)]).to_tokens(tokens);

        states.to_tokens(tokens);
        let name = widget.as_ref().unwrap().node.name();
        quote! { let #name = move | }.to_tokens(tokens);
        if let Some(ctx_name) = init.as_ref().and_then(|i| i.ctx_name.as_ref()) {
          ctx_name.to_tokens(tokens);
        } else {
          ctx_ident().to_tokens(tokens)
        };
        quote! { : &BuildCtx|}.to_tokens(tokens);

        Brace::default().surround(tokens, |tokens| {
          if ctx.has_guards_data {
            let guards_vec = guard_vec_ident();
            quote! { let mut #guards_vec: Vec<AnonymousData> = vec![]; }.to_tokens(tokens);
          }
          init.to_tokens(tokens);

          // deep first declare named obj by their dependencies
          // circular may exist widget attr follow widget self to init.
          sorted_named_objs.iter().for_each(|name| {
            if let Some(obj) = named_objs.get(name) {
              obj.to_tokens(tokens)
            }
          });

          let w = widget.as_ref().unwrap();
          w.gen_node_objs(tokens);
          finally.to_tokens(tokens);

          if ctx.has_guards_data {
            quote! { widget_attach_data }.to_tokens(tokens);
            Paren::default().surround(tokens, |tokens| {
              w.gen_compose_node(named_objs, tokens);
              let guards_vec = guard_vec_ident();
              quote! { .into_widget(),#guards_vec }.to_tokens(tokens)
            });
          } else {
            w.gen_compose_node(named_objs, tokens)
          }
        });
        quote! { ; #name.into_widget() }.to_tokens(tokens);
      });
    });
  }

  pub fn collect_warnings(&mut self, ctx: &VisitCtx) { self.collect_unused_declare_obj(ctx); }

  pub fn circle_detect(&mut self) {
    fn used_part_iter(obj: &DeclareObj) -> impl Iterator<Item = UsedPart> + '_ {
      obj.fields.iter().flat_map(|f| match &f.value {
        FieldValue::Expr(e) => e.used_name_info.used_part(Some(&f.member)),
        // embed object must be an anonymous object never construct a circle.
        FieldValue::Obj(_) => None,
      })
    }

    let mut depends = BTreeMap::default();
    let Self { named_objs, errors, .. } = self;
    named_objs.iter().for_each(|(name, obj)| {
      let obj_used: ObjectUsed = match obj {
        NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => used_part_iter(obj).collect(),
      };
      if !obj_used.is_empty() {
        depends.insert(name, obj_used);
      }
    });

    #[derive(PartialEq, Eq, Debug, Clone, Copy)]
    enum CheckState {
      Checking,
      Checked,
    }

    let mut circles = vec![];
    let mut check_info: HashMap<_, _, RandomState> = HashMap::default();
    let mut edges: Vec<ObjectUsedPath> = vec![];
    depends.keys().for_each(|name| {
      loop {
        let node = edges.last().map_or(*name, |e| e.used_obj);
        let check_state = check_info.get(node).copied();
        if check_state.is_none() {
          let edge_size = edges.len();
          if let Some(depends) = depends.get(node) {
            edges.extend(depends.used_full_path_iter(node));
          }
          if edges.len() > edge_size {
            check_info.insert(node, CheckState::Checking);
          } else {
            check_info.insert(node, CheckState::Checked);
          }
          continue;
        }

        if let Some(CheckState::Checking) = check_state {
          let mut circle = vec![edges.last().cloned().unwrap()];
          for edge in edges.iter().rev() {
            let circle_last = circle.last().unwrap();
            if circle_last.obj != edge.obj {
              if edge.used_obj == circle_last.obj {
                circle.push(edge.clone());
              } else {
                break;
              }
            }
          }

          circles.push(circle);
        }

        let edge = edges.pop();
        let next = edges.last();
        match (edge, next) {
          (e, None) => {
            check_info.insert(node, CheckState::Checked);
            if let Some(e) = e {
              check_info.insert(e.obj, CheckState::Checked);
            }
            break;
          }
          (None, Some(_)) => unreachable!(),
          (Some(e), Some(n)) => {
            if e.obj != n.obj {
              check_info.insert(e.obj, CheckState::Checked);
            }
          }
        }
      }
    });

    circles.iter().for_each(|path| {
      let circle =
        DeclareError::CircleDepends(path.iter().map(|c| c.to_used_path(named_objs)).collect());
      errors.push(circle);
    });
  }

  fn order_named_objs(&self) -> Vec<Ident> {
    let mut orders = vec![];
    let mut visit_state: HashSet<_, RandomState> = HashSet::default();
    self
      .named_objs
      .names()
      .for_each(|name| self.obj_deep_first(name, &mut visit_state, &mut orders));
    orders
  }

  fn obj_deep_first<'a>(
    &'a self,
    name: &'a Ident,
    visit_state: &mut HashSet<&'a Ident, RandomState>,
    orders: &mut Vec<Ident>,
  ) {
    if !visit_state.contains(name) {
      visit_state.insert(name);
      if let Some(obj) = self.named_objs.get(name) {
        match obj {
          NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => obj
            .fields
            .iter()
            .for_each(|f| self.deep_in_field(f, visit_state, orders)),
        }

        orders.push(name.clone());
      }
    }
  }

  fn deep_in_field<'a>(
    &'a self,
    f: &'a Field,
    visit_state: &mut HashSet<&'a Ident, RandomState>,
    orders: &mut Vec<Ident>,
  ) {
    match &f.value {
      FieldValue::Expr(e) => {
        if let Some(all) = e.used_name_info.all_used() {
          all.for_each(|name| self.obj_deep_first(name, visit_state, orders))
        }
      }
      FieldValue::Obj(obj) => obj
        .fields
        .iter()
        .for_each(|f| self.deep_in_field(f, visit_state, orders)),
    }
  }

  fn collect_unused_declare_obj(&mut self, ctx: &VisitCtx) {
    let used_ids = ctx
      .used_objs
      .iter()
      .map(|(name, info)| {
        if let Some(builtin) = info.builtin.as_ref() {
          &builtin.src_name
        } else {
          name
        }
      })
      .collect::<HashSet<_, ahash::RandomState>>();
    self
      .named_objs
      .iter()
      .filter(|(name, obj)| {
        // Needn't check builtin named widget, shared id with host in user side.
        matches!(obj, NamedObj::Host(_))
          && !used_ids.contains(name)
          && !name.to_string().starts_with('_')
      })
      .for_each(|(name, _)| {
        self
          .warnings
          .push(DeclareWarning::UnusedName(name.span().unwrap()));
      });
  }
}

impl ToTokens for FieldValue {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      FieldValue::Expr(e) => e.to_tokens(tokens),
      FieldValue::Obj(obj) => Brace(obj.span()).surround(tokens, |tokens| {
        obj.to_tokens(tokens);
        obj.name.to_tokens(tokens)
      }),
    }
  }
}

impl ToTokens for DeclareObj {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let DeclareObj { ty, name, watch_stmts, .. } = self;
    let span = ty.span();
    if watch_stmts.is_empty() {
      self.build_tokens(tokens);
    } else {
      quote_spanned! { span => let #name = }.to_tokens(tokens);
      Brace(ty.span()).surround(tokens, |tokens| {
        watch_stmts.iter().for_each(|f| {
          f.field_fn.to_tokens(tokens);
        });
        self.build_tokens(tokens);
        watch_stmts.iter().for_each(|f| {
          f.watch_update.to_tokens(tokens);
        });
        name.to_tokens(tokens);
      });
      Semi(span).to_tokens(tokens);
    }
  }
}

impl DeclareObj {
  fn build_tokens(&self, tokens: &mut TokenStream) {
    let DeclareObj {
      fields,
      ty,
      name,
      watch_stmts,
      stateful,
      used_name_info,
    } = self;
    let span = ty.span();
    quote_spanned! { span => let #name = }.to_tokens(tokens);
    let builder = |tokens: &mut TokenStream| {
      quote_spanned! { span => #ty::declare_builder() }.to_tokens(tokens);
      fields.iter().for_each(|f| {
        let Field { member, value, .. } = f;
        Dot(value.span()).to_tokens(tokens);
        member.to_tokens(tokens);
        Paren(value.span()).surround(tokens, |tokens| value.to_tokens(tokens))
      });
      let build_ctx = ctx_ident();
      tokens.extend(quote_spanned! { span => .build(#build_ctx) });
      let is_stateful = *stateful || !watch_stmts.is_empty();
      if is_stateful {
        quote_spanned! { span => .into_stateful() }.to_tokens(tokens);
      }
    };
    if used_name_info.ref_widgets().is_some() {
      Brace(span).surround(tokens, |tokens| {
        used_name_info.prepend_bundle_refs(tokens);
        builder(tokens);
      })
    } else {
      builder(tokens);
    }

    Semi(span).to_tokens(tokens);
  }
}

impl ToTokens for StateField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let StateField { member, expr, .. } = self;
    syn::token::Let(member.span()).to_tokens(tokens);
    member.to_tokens(tokens);
    quote_spanned!(member.span() => : Stateful<_> =  ).to_tokens(tokens);
    expr.to_tokens(tokens);
    Semi(member.span()).to_tokens(tokens);
  }
}

impl ToTokens for States {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self
      .states
      .iter()
      .filter(|f| f.colon_token.is_some())
      .for_each(|field| field.to_tokens(tokens));
  }
}

impl WidgetNode {
  fn gen_node_objs(&self, tokens: &mut TokenStream) {
    let WidgetNode { node: parent, children } = self;
    if let ComposeItem::ChainObjs(objs) = parent {
      objs.iter().for_each(|obj| obj.to_tokens(tokens));
    }

    children.iter().for_each(|node| node.gen_node_objs(tokens));
  }

  fn gen_compose_node(&self, named_objs: &NamedObjMap, tokens: &mut TokenStream) {
    fn recursive_compose(
      nodes: &[&Ident],
      children: &[WidgetNode],
      named_objs: &NamedObjMap,
      tokens: &mut TokenStream,
    ) {
      let first = &nodes[0];
      first.to_tokens(tokens);
      let span = first.span();

      if nodes.len() > 1 {
        quote_spanned! { span => .with_child}.to_tokens(tokens);
        Paren(span).surround(tokens, |tokens| {
          recursive_compose(&nodes[1..], children, named_objs, tokens);
        });
      } else {
        children.iter().for_each(|c| {
          quote_spanned!(span => .with_child).to_tokens(tokens);
          Paren(span).surround(tokens, |tokens| c.gen_compose_node(named_objs, tokens))
        });
      }
    }

    let WidgetNode { node, children } = self;
    let nodes = node.node_compose_list(named_objs);
    recursive_compose(&nodes, children, named_objs, tokens);
  }
}

impl ComposeItem {
  fn node_compose_list<'a>(&'a self, named_objs: &'a NamedObjMap) -> SmallVec<[&'a Ident; 1]> {
    let mut list = smallvec![];
    match self {
      ComposeItem::ChainObjs(objs) => {
        assert!(!objs.is_empty());
        list.extend(objs.iter().map(|obj| &obj.name));
      }
      ComposeItem::Id(name) => {
        WIDGETS
          .iter()
          .rev()
          .filter_map(|builtin| {
            let var_name = builtin_var_name(name, name.span(), builtin.ty);
            named_objs.get_name_obj(&var_name)
          })
          .for_each(|(var_name, obj)| match obj {
            NamedObj::Builtin { .. } => list.push(var_name),
            NamedObj::Host(..) => unreachable!("builtin object type not match."),
          });

        list.push(name);
      }
    };
    list
  }
}

impl ToTokens for NamedObj {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => obj.to_tokens(tokens),
    }
  }
}

impl States {
  pub fn track_names(&self) -> impl Iterator<Item = &Ident> {
    self.states.iter().map(|f| &f.member)
  }
}

impl ToTokens for InitStmts {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    if let Some(refs) = self.used_name_info.ref_widgets() {
      let refs = refs.map(|name| {
        quote_spanned! { name.span() =>
          #[allow(unused_mut)]
          let mut #name = #name.state_ref();
        }
      });
      tokens.append_all(refs);
    }
    tokens.append_all(&self.stmts);
    if let Some(name) = self.ctx_name.as_ref() {
      let inner_name = ctx_ident();
      quote! { let #inner_name = #name; }.to_tokens(tokens);
    }
  }
}

impl ToTokens for FinallyBlock {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self.brace_token.surround(tokens, |tokens| {
      self.used_name_info.prepend_bundle_refs(tokens);
      tokens.append_all(&self.stmts)
    })
  }
}
impl ToTokens for FinallyStmt {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      FinallyStmt::Stmt(stmt) => stmt.to_tokens(tokens),
      FinallyStmt::Obj(obj) => obj.to_tokens(tokens),
    }
  }
}

impl ToTokens for PropMacro {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { prop, lerp_fn, .. } = self;
    if let Some(lerp_fn) = lerp_fn {
      let span = lerp_fn.span();
      quote_spanned!(span => LerpProp::new (#prop, #lerp_fn)).to_tokens(tokens);
    } else {
      prop.to_tokens(tokens)
    }
  }
}

impl ToTokens for Property {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      Property::Name(name) => quote_spanned! {name.span() =>
        Prop::new(#name.clone_stateful(), |v| v.clone(), |this, v| *this = v)
      }
      .to_tokens(tokens),
      Property::Member { target, dot, member } => quote_spanned! {
        target.span().join(member.span()).unwrap() =>
        Prop::new(
          #target.clone_stateful(),
          |this| this #dot #member.clone(),
          |this, v| this #dot #member = v
        )
      }
      .to_tokens(tokens),
    }
  }
}
