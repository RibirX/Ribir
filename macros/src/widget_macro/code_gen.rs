use crate::{
  declare_derive::declare_field_name,
  error::{DeclareError, DeclareWarning},
};
use ahash::RandomState;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::collections::{BTreeMap, HashMap, HashSet};
use syn::{
  parse_quote_spanned,
  spanned::Spanned,
  token::{Brace, Dot, Paren, Semi},
  Ident,
};

use super::{
  builtin_var_name, capture_widget, ctx_ident,
  desugar::{
    ComposeItem, DeclareObj, Field, FieldValue, NamedObj, NamedObjMap, SubscribeItem, WidgetNode,
  },
  parser::{Track, TrackField},
  Desugared, ObjectUsed, ObjectUsedPath, ScopeUsedInfo, TrackExpr, UsedPart, UsedType, VisitCtx,
  WIDGETS,
};

impl Desugared {
  pub fn gen_code(&mut self) -> TokenStream {
    self.circle_detect();
    let mut tokens = quote! {};
    if !self.errors.is_empty() {
      Brace::default().surround(&mut tokens, |tokens| {
        self
          .errors
          .iter()
          .for_each(|err| err.into_compile_error(tokens));
        quote! { Void }.to_tokens(tokens);
      });

      return tokens;
    }

    let sorted_named_objs = self.order_named_objs();
    Paren::default().surround(&mut tokens, |tokens| {
      let ctx_name = ctx_ident(Span::call_site());
      quote! { move |#ctx_name: &mut BuildCtx| }.to_tokens(tokens);
      Brace::default().surround(tokens, |tokens| {
        // deep first declare named obj by their dependencies
        // circular may exist widget attr follow widget self to init.
        sorted_named_objs.iter().for_each(|name| {
          if let Some(obj) = self.named_objs.get(name) {
            obj.to_tokens(tokens)
          }
        });

        self.stmts.iter().for_each(|item| item.to_tokens(tokens));
        let w = self.widget.as_ref().unwrap();
        w.gen_node_objs(tokens);
        w.gen_compose_node(&self.named_objs, tokens);
        quote! { .into_widget() }.to_tokens(tokens);
      })
    });
    quote! { .into_widget() }.to_tokens(&mut tokens);

    let track = self.track.as_ref();
    if track.map_or(false, Track::has_def_names) {
      tokens = quote! {{
        #track
        #tokens
      }};
    }

    self.warnings.iter().for_each(|w| w.emit_warning());

    tokens
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

    let mut depends = BTreeMap::new();
    self.named_objs.iter().for_each(|(name, obj)| {
      let obj_used: ObjectUsed = match obj {
        NamedObj::Host(obj) => used_part_iter(obj).collect(),
        NamedObj::Builtin { objs, .. } => objs.iter().flat_map(used_part_iter).collect(),
      };
      if !obj_used.is_empty() {
        depends.insert(name, obj_used);
      }
    });

    self.stmts.iter().for_each(|item| match item {
      SubscribeItem::Obj(_) => {
        // embed object must be a antonymous object.
      }
      SubscribeItem::ObserveModifyDo { observe, subscribe_do } => {
        if let Some(observes) = observe.used_name_info.directly_used_widgets() {
          let used = subscribe_do.used_name_info.used_part(None);
          if let Some(used) = used {
            let used = ObjectUsed(Box::new([used]));
            observes.for_each(|name| {
              depends.insert(name, used.clone());
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
          // todo!("start from node");
          edges.iter().rev().for_each(|edge| {
            if circle.last().map_or(false, |p| p.obj != edge.obj) {
              circle.push(edge.clone());
            }
          });
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
      let circle = DeclareError::CircleDepends(
        path
          .iter()
          .map(|c| c.to_used_path(&self.named_objs))
          .collect(),
      );
      self.errors.push(circle);
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
          NamedObj::Host(obj) => obj
            .fields
            .iter()
            .for_each(|f| self.deep_in_field(f, visit_state, orders)),
          NamedObj::Builtin { objs, .. } => objs
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
    self
      .named_objs
      .iter()
      .filter_map(|(name, obj)| {
        // Needn't check builtin named widget, shared id with host in user side.
        matches!(obj, NamedObj::Host(_)).then(|| name)
      })
      .filter(|name| !ctx.used_widgets.contains_key(name) && !name.to_string().starts_with('_'))
      .for_each(|name| {
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
    let DeclareObj { fields, ty, name, stateful, .. } = self;
    let whole_used_info = self.whole_used_info();
    let span = ty.span();
    quote_spanned! { span => let #name = }.to_tokens(tokens);
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
                  move |(_, after)| #name.state_ref().#declare_set(after)
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
    let observe_span = observe.span();
    upstream.to_tokens(tokens);
    let mut expr_value = quote! {};
    observe
      .used_name_info
      .value_expr_surround_refs(&mut expr_value, observe_span, |tokens| {
        observe.to_tokens(tokens)
      });

    let captures = observe
      .used_name_info
      .all_used()
      .expect("if upstream is not none, must used some widget")
      .map(capture_widget);

    quote_spanned! { observe.span() =>
      .filter(|s| s.contains(ChangeScope::DATA))
      .scan_initial({
          let v = #expr_value;
          (v.clone(), v)
        }, {
          #(#captures)*
          move |(_, after), _| { (after, #expr_value)}
      })
    }
    .to_tokens(tokens);
    if change_only {
      quote_spanned! { observe.span() =>
        .filter(|(before, after)| before != after)
      }
      .to_tokens(tokens);
    }

    let subscribe_span = subscribe_do.span();
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
    parent.gen_compose_item(named_objs, tokens);

    if !children.is_empty() {
      children.iter().for_each(|node| {
        quote! {.have_child}.to_tokens(tokens);
        Paren::default().surround(tokens, |tokens| {
          node.gen_compose_node(named_objs, tokens);
        });
      });
    }
  }
}

impl ComposeItem {
  fn gen_compose_item(&self, named_objs: &NamedObjMap, tokens: &mut TokenStream) {
    match self {
      ComposeItem::ChainObjs(objs) => {
        assert!(objs.len() > 0);
        objs[0].name.to_tokens(tokens);
        objs[1..].iter().for_each(|obj| {
          let name = &obj.name;
          quote! {.have_child(#name)}.to_tokens(tokens);
        })
      }
      ComposeItem::Id(name) => {
        let builtin = WIDGETS
          .iter()
          .rev()
          .filter_map(|builtin| {
            let var_name = builtin_var_name(name, &builtin.ty);
            named_objs.contains(&var_name).then(|| var_name)
          })
          .collect::<Vec<_>>();
        if !builtin.is_empty() {
          let first = &builtin[0];
          first.to_tokens(tokens);
          builtin[1..]
            .iter()
            .for_each(|name| quote! { .have_child(#name)}.to_tokens(tokens));
          quote! { .have_child(#name)}.to_tokens(tokens)
        } else {
          name.to_tokens(tokens);
        }
      }
    }
  }
}

impl ToTokens for NamedObj {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      NamedObj::Host(obj) => obj.to_tokens(tokens),
      NamedObj::Builtin { objs, .. } => {
        let most_inner = objs.last().unwrap();
        most_inner.to_tokens(tokens);
        objs.iter().rev().skip(1).for_each(|obj| {
          let name = &obj.name;
          quote_spanned! {obj.span() =>  let #name = }.to_tokens(tokens);
          Brace(obj.span()).surround(tokens, |tokens| {
            quote_spanned! { obj.span() =>  let tmp = #name; }.to_tokens(tokens);
            obj.to_tokens(tokens);
            quote_spanned! {obj.span() => #name.have_child(tmp)}.to_tokens(tokens);
          });
          Semi(obj.span()).to_tokens(tokens);
        });
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
