use crate::{
  declare_derive::declare_field_name,
  error::{DeclareError, DeclareWarning},
};
use ahash::RandomState;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use smallvec::{smallvec, SmallVec};
use std::collections::{BTreeMap, HashMap, HashSet};
use syn::{
  parse_quote_spanned,
  spanned::Spanned,
  token::{Brace, Comma, Dot, Paren, Semi},
  Ident,
};

use super::{
  builtin_var_name, capture_widget, ctx_ident,
  desugar::{
    ComposeItem, DeclareObj, Field, FieldValue, NamedObj, NamedObjMap, SubscribeItem, WidgetNode,
  },
  guard_ident, guard_vec_ident,
  parser::{Env, Track, TrackField},
  Desugared, ObjectUsed, ObjectUsedPath, ScopeUsedInfo, TrackExpr, UsedPart, UsedType, VisitCtx,
  WIDGETS,
};

impl ToTokens for Desugared {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    if !self.errors.is_empty() {
      Brace::default().surround(tokens, |tokens| {
        self
          .errors
          .iter()
          .for_each(|err| err.into_compile_error(tokens));
        quote! { Void }.to_tokens(tokens);
      });

      return;
    }
    let Self { track, warnings, .. } = &*self;
    if track.as_ref().map_or(false, Track::has_def_names) {
      Brace::default().surround(tokens, |tokens| {
        track.to_tokens(tokens);
        self.inner_tokens(tokens);
      })
    } else {
      self.inner_tokens(tokens);
    }

    warnings.iter().for_each(|w| w.emit_warning());
  }
}

impl Desugared {
  fn inner_tokens(&self, tokens: &mut TokenStream) {
    let Self { env, named_objs, stmts, widget, .. } = &self;
    let sorted_named_objs = self.order_named_objs();
    Paren::default().surround(tokens, |tokens| {
      let ctx_name = ctx_ident(Span::call_site());
      quote! { move |#ctx_name: &BuildCtx| }.to_tokens(tokens);
      Brace::default().surround(tokens, |tokens| {
        let guards_vec = guard_vec_ident();
        quote! {
         let mut #guards_vec: Vec<SubscriptionGuard<Box<dyn SubscriptionLike>>> = vec![];
        }
        .to_tokens(tokens);
        env.to_tokens(tokens);
        // deep first declare named obj by their dependencies
        // circular may exist widget attr follow widget self to init.
        sorted_named_objs.iter().for_each(|name| {
          if let Some(obj) = named_objs.get(name) {
            obj.to_tokens(tokens)
          }
        });

        stmts.iter().for_each(|item| item.to_tokens(tokens));
        let w = widget.as_ref().unwrap();
        w.gen_node_objs(tokens);
        let name = w.parent.name();
        quote! { let mut #name = }.to_tokens(tokens);
        w.gen_compose_node(named_objs, tokens);
        quote! {
          .into_widget();
          if !#guards_vec.is_empty() {
            #name = compose_child_as_data_widget(#name, StateWidget::Stateless(#guards_vec));
          }
          #name
        }
        .to_tokens(tokens);
      })
    });
    quote! { .into_widget() }.to_tokens(tokens);
  }

  pub fn collect_warnings(&mut self, ctx: &VisitCtx) {
    self.collect_unused_declare_obj(ctx);
    self.collect_observe_nothing();
  }

  pub fn circle_detect(&mut self) {
    fn used_part_iter(obj: &DeclareObj) -> impl Iterator<Item = UsedPart> + '_ {
      obj.fields.iter().flat_map(|f| match &f.value {
        FieldValue::Expr(e) => e.used_name_info.used_part(Some(&f.member)),
        // embed object must be an anonymous object never construct a circle.
        FieldValue::Obj(_) => None,
      })
    }

    let mut depends = BTreeMap::default();
    let Self { named_objs, stmts, errors, .. } = self;
    named_objs.iter().for_each(|(name, obj)| {
      let obj_used: ObjectUsed = match obj {
        NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => used_part_iter(obj).collect(),
        NamedObj::DuplicateListener { objs, .. } => objs.iter().flat_map(used_part_iter).collect(),
      };
      if !obj_used.is_empty() {
        depends.insert(name, obj_used);
      }
    });

    stmts.iter().for_each(|item| match item {
      SubscribeItem::Obj(_) => {
        // embed object must be a antonymous object.
      }
      SubscribeItem::ObserveModifyDo { observe, subscribe_do } => {
        if let Some(observes) = observe.used_name_info.directly_used_widgets() {
          let used = subscribe_do.used_name_info.used_part(None);
          if let Some(used) = used {
            let used = ObjectUsed(vec![used]);
            observes.for_each(|name| {
              depends
                .entry(name)
                .and_modify(|obj| obj.0.extend(used.0.clone()))
                .or_insert_with(|| used.clone());
            });
          }
        }
      }
      SubscribeItem::ObserveChangeDo { .. } => {
        // change event will compare the value use to break depends.
      }
      SubscribeItem::LetVar { .. } => {}
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
        let node = edges.last().map_or(*name, |e| &e.used_obj);
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
          NamedObj::DuplicateListener { objs, .. } => objs
            .iter()
            .flat_map(|obj| obj.fields.iter())
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
    let on_event_used = self
      .named_objs
      .objs()
      .filter_map(|obj| match obj {
        NamedObj::Host(_) => None,
        NamedObj::Builtin { src_name, obj } => obj.desugar_from_on_event.then_some(src_name),
        NamedObj::DuplicateListener { src_name, .. } => Some(src_name),
      })
      .collect::<HashSet<_, ahash::RandomState>>();

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
          && !on_event_used.contains(name)
      })
      .for_each(|(name, _)| {
        self
          .warnings
          .push(DeclareWarning::UnusedName(name.span().unwrap()));
      });
  }

  pub fn collect_observe_nothing(&mut self) {
    self.stmts.iter().for_each(|stmt| match stmt {
      SubscribeItem::ObserveModifyDo { observe, .. }
      | SubscribeItem::ObserveChangeDo { observe, .. } => {
        if observe.used_name_info.directly_used_widgets().is_none() {
          self
            .warnings
            .push(DeclareWarning::ObserveIsConst(observe.span().unwrap()));
        }
      }
      _ => {}
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
    let DeclareObj { fields, ty, name, .. } = self;
    let span = ty.span();
    quote_spanned! { span => let #name = }.to_tokens(tokens);
    self.gen_build_tokens(tokens);
    Semi(span).to_tokens(tokens);

    fields
      .iter()
      .for_each(|Field { member, value }| match value {
        FieldValue::Expr(expr) => {
          if expr.used_name_info.directly_used_widgets().is_some() {
            let declare_set = declare_field_name(member);

            let mut used_name_info = ScopeUsedInfo::default();
            used_name_info.add_used(name.clone(), UsedType::MOVE_CAPTURE);
            let on_change_do = SubscribeItem::ObserveModifyDo {
              observe: expr.clone(),
              subscribe_do: TrackExpr {
                expr: parse_quote_spanned! { member.span() => {
                  let #name = #name.clone_stateful();
                  move |v| #name.state_ref().#declare_set(v)
                }},
                used_name_info,
              },
            };
            on_change_do.to_tokens(tokens);
          }
        }
        FieldValue::Obj(_) => {
          // directly obj needn't subscribe anything, its fields directly
          // subscribe.
        }
      });
  }
}

impl DeclareObj {
  pub fn whole_used_info(&self) -> ScopeUsedInfo {
    self
      .fields
      .iter()
      .fold(ScopeUsedInfo::default(), |mut acc, f| {
        if let FieldValue::Expr(e) = &f.value {
          acc.merge(&e.used_name_info)
        }
        acc
      })
  }

  pub fn depends_any_other(&self) -> bool {
    self.fields.iter().any(|f| match &f.value {
      FieldValue::Expr(e) => e.used_name_info.directly_used_widgets().is_some(),
      FieldValue::Obj(_) => false,
    })
  }

  fn gen_as_value(&self, tokens: &mut TokenStream) {
    if self.depends_any_other() {
      Brace(self.span()).surround(tokens, |tokens| {
        self.to_tokens(tokens);
        self.name.to_tokens(tokens)
      })
    } else {
      self.gen_build_tokens(tokens);
    }
  }

  fn gen_build_tokens(&self, tokens: &mut TokenStream) {
    let DeclareObj { fields, ty, stateful, .. } = self;
    let whole_used_info = self.whole_used_info();
    let span = ty.span();
    whole_used_info.value_expr_surround_refs(tokens, span, |tokens| {
      tokens.extend(quote_spanned! { span => <#ty as Declare>::builder() });
      fields.iter().for_each(|f| {
        let Field { member, value, .. } = f;
        Dot(value.span()).to_tokens(tokens);
        member.to_tokens(tokens);
        Paren(value.span()).surround(tokens, |tokens| value.to_tokens(tokens))
      });
      let build_ctx = ctx_ident(ty.span());
      tokens.extend(quote_spanned! { span => .build(#build_ctx) });
      let is_stateful = *stateful || whole_used_info.directly_used_widgets().is_some();
      if is_stateful {
        tokens.extend(quote_spanned! { span => .into_stateful() });
      }
    });
  }
}

impl ToTokens for TrackField {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let TrackField { member, expr, .. } = self;
    syn::token::Let(member.span()).to_tokens(tokens);
    member.to_tokens(tokens);
    quote_spanned!(member.span() => : Stateful<_> =  ).to_tokens(tokens);
    expr.to_tokens(tokens);
    Semi(member.span()).to_tokens(tokens);
  }
}

impl ToTokens for Track {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    self
      .track_externs
      .iter()
      .filter(|f| f.colon_token.is_some())
      .for_each(|field| field.to_tokens(tokens));
  }
}

impl ToTokens for Env {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.stmts.stmts.iter().for_each(|s| s.to_tokens(tokens));
  }
}

impl ToTokens for SubscribeItem {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      SubscribeItem::Obj(obj) => obj.to_tokens(tokens),
      SubscribeItem::ObserveModifyDo { observe, subscribe_do } => {
        subscribe_modify(observe, subscribe_do, false, tokens)
      }
      SubscribeItem::ObserveChangeDo { observe, subscribe_do } => {
        subscribe_modify(observe, subscribe_do, true, tokens)
      }
      SubscribeItem::LetVar { name, value } => {
        quote_spanned! {name.span() => let #name = }.to_tokens(tokens);
        value
          .used_name_info
          .value_expr_surround_refs(tokens, value.span(), |tokens| {
            value.to_tokens(tokens);
          });
        Semi(name.span()).to_tokens(tokens);
      }
    }
  }
}

fn subscribe_modify(
  observe: &TrackExpr,
  subscribe_do: &TrackExpr,
  change_only: bool,
  tokens: &mut TokenStream,
) {
  if let Some(upstream) = observe.upstream_tokens() {
    let guard = guard_ident();
    let subscribe_span = subscribe_do.span();
    quote_spanned! { subscribe_span => let #guard =  }.to_tokens(tokens);

    let observe_span = observe.span();
    upstream.to_tokens(tokens);
    let mut expr_value = quote! {};
    observe
      .used_name_info
      .value_expr_surround_refs(&mut expr_value, observe_span, |tokens| {
        observe.to_tokens(tokens)
      });

    quote_spanned! { observe.span() =>
      .filter(|s| s.contains(ChangeScope::DATA))
    }
    .to_tokens(tokens);
    let captures = observe
      .used_name_info
      .all_used()
      .expect("if upstream is not none, must used some widget")
      .map(capture_widget);
    if change_only {
      quote_spanned! { observe.span() =>
        .scan_initial(
          (#expr_value, #expr_value),
          {
            #(#captures)*
            move |(_, after), _| { (after, #expr_value)}
          }
        )
        .filter(|(before, after)| before != after)
      }
      .to_tokens(tokens);
    } else {
      quote_spanned! { observe.span() =>
        .map(
          {
            #(#captures)*
            move |_| #expr_value
          }
        )
      }
      .to_tokens(tokens);
    }

    quote_spanned! {subscribe_span => .subscribe}.to_tokens(tokens);
    Paren(subscribe_span).surround(tokens, |tokens| {
      if subscribe_do.used_name_info.refs_widgets().is_some() {
        Brace(subscribe_span).surround(tokens, |tokens| {
          subscribe_do.used_name_info.refs_surround(tokens, |tokens| {
            subscribe_do.to_tokens(tokens);
          });
        })
      } else {
        subscribe_do.to_tokens(tokens);
      }
    });
    Semi(subscribe_span).to_tokens(tokens);

    let guard_vec = guard_vec_ident();
    quote_spanned! { subscribe_span =>
      let #guard: Box<dyn SubscriptionLike> = Box::new(#guard.into_inner());
      #guard_vec.push(SubscriptionGuard::new(#guard));
    }
    .to_tokens(tokens);
  }
}

impl WidgetNode {
  fn gen_node_objs(&self, tokens: &mut TokenStream) {
    let WidgetNode { parent, children } = self;
    if let ComposeItem::ChainObjs(objs) = parent {
      objs.iter().for_each(|obj| obj.to_tokens(tokens));
    }

    children.iter().for_each(|node| node.gen_node_objs(tokens));
  }

  fn gen_compose_node(&self, named_objs: &NamedObjMap, tokens: &mut TokenStream) {
    let WidgetNode { parent, children } = self;
    let mut compose_list = parent.node_compose_list(named_objs);

    if !children.is_empty() {
      let last = compose_list
        .last_mut()
        .expect("must at least have one obj to compose.");

      let span = last.span();
      quote_spanned! {span => .with_child}.to_tokens(last);
      Paren(span).surround(last, |tokens| {
        if children.len() > 1 {
          Paren(span).surround(tokens, |tokens| {
            children.iter().for_each(|node| {
              node.gen_compose_node(named_objs, tokens);
              Comma(span).to_tokens(tokens);
            });
          });
        } else {
          children[0].gen_compose_node(named_objs, tokens);
        }
      });
    }
    compose_list[0].to_tokens(tokens);
    recursive_compose(compose_list.into_iter().skip(1), tokens);
  }
}

fn recursive_compose(mut chain: impl Iterator<Item = TokenStream>, tokens: &mut TokenStream) {
  if let Some(current) = chain.next() {
    let span = current.span();
    quote_spanned! {span => .with_child}.to_tokens(tokens);
    Paren(span).surround(tokens, |tokens| {
      current.to_tokens(tokens);
      recursive_compose(chain, tokens);
    });
  }
}

impl ComposeItem {
  fn node_compose_list(&self, named_objs: &NamedObjMap) -> SmallVec<[TokenStream; 1]> {
    let mut list = smallvec![];
    match self {
      ComposeItem::ChainObjs(objs) => {
        assert!(objs.len() > 0);
        list.extend(objs.iter().map(|obj| obj.name.clone().into_token_stream()));
      }
      ComposeItem::Id(name) => {
        WIDGETS
          .iter()
          .rev()
          .filter_map(|builtin| {
            let var_name = builtin_var_name(name, name.span(), &builtin.ty);
            named_objs.get_name_obj(&var_name)
          })
          .for_each(|(var_name, obj)| match obj {
            NamedObj::DuplicateListener { objs, .. } => {
              list.extend(objs.iter().map(|obj| {
                let mut obj_tokens = quote! {};
                obj.gen_as_value(&mut obj_tokens);
                obj_tokens
              }));
            }
            NamedObj::Builtin { .. } => list.push(quote! {#var_name}),
            NamedObj::Host(..) => unreachable!("builtin object type not match."),
          });

        list.push(quote! {#name});
      }
    };
    list
  }
}

impl ToTokens for NamedObj {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      NamedObj::Host(obj) | NamedObj::Builtin { obj, .. } => obj.to_tokens(tokens),
      NamedObj::DuplicateListener { .. } => {
        // duplicated listener should not allow by others, directly do recursive
        // compose in later.
      }
    }
  }
}

impl Track {
  pub fn has_def_names(&self) -> bool { self.track_externs.iter().any(|f| f.colon_token.is_some()) }

  pub fn track_names(&self) -> impl Iterator<Item = &Ident> {
    self.track_externs.iter().map(|f| &f.member)
  }
}
