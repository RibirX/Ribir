use proc_macro2::Span;
use quote::{quote, ToTokens};
use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;
use syn::{parse_quote, parse_quote_spanned, spanned::Spanned, Expr, ExprPath, Ident, Path};

use super::{
  child_variable, is_listener,
  parser::{
    Animate, AnimateTransitionValue, DataFlow, DeclareField, DeclareWidget, FromStateField, Id,
    Item, MacroSyntax, MemberPath, Observe, OnEventDo, QuickDo, Track, Transition, TransitionField,
  },
  ribir_suffix_variable, ribir_variable, TrackExpr, FIELD_WIDGET_TYPE, WIDGETS,
};
use crate::{
  error::{DeclareError, DeclareWarning},
  widget_macro::builtin_var_name,
};

#[derive(Default)]
pub struct NamedObjMap(HashMap<Ident, NamedObj, ahash::RandomState>);

pub const CHANGE: &str = "change";
pub const MODIFY: &str = "modify";
pub const ID: &str = "id";
pub struct Desugared {
  pub track: Option<Track>,
  pub named_objs: NamedObjMap,
  pub stmts: Vec<SubscribeItem>,
  pub widget: Option<WidgetNode>,
  pub errors: Vec<DeclareError>,
  pub warnings: Vec<DeclareWarning>,
}

pub enum SubscribeItem {
  Obj(DeclareObj),
  ObserveModifyDo {
    observe: TrackExpr,
    subscribe_do: TrackExpr,
  },
  ObserveChangeDo {
    observe: TrackExpr,
    subscribe_do: TrackExpr,
  },
  LetVar {
    name: Ident,
    value: TrackExpr,
  },
}
#[derive(Debug, Clone)]
pub struct DeclareObj {
  pub ty: Path,
  pub name: Ident,
  pub fields: SmallVec<[Field; 1]>,
  pub stateful: bool,
  pub desugar_from_on_event: bool,
}

#[derive(Debug)]
pub struct BuiltinObj {
  pub obj: DeclareObj,
  pub src_name: Option<Ident>,
}

#[derive(Debug)]
pub enum NamedObj {
  Host(DeclareObj),
  Builtin {
    src_name: Ident,
    obj: DeclareObj,
  },
  DuplicateListener {
    src_name: Ident,
    objs: Vec<DeclareObj>,
  },
}

#[derive(Debug, Clone)]
pub struct Field {
  pub member: Ident,
  pub value: FieldValue,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
  Expr(TrackExpr),
  Obj(Box<DeclareObj>),
}
pub struct WidgetNode {
  pub parent: ComposeItem,
  pub children: Vec<WidgetNode>,
}

pub enum ComposeItem {
  ChainObjs(SmallVec<[DeclareObj; 1]>),
  Id(Ident),
}

impl MacroSyntax {
  pub fn desugar(self) -> Desugared {
    let named_objs = NamedObjMap::default();
    let MacroSyntax { track, widget, items } = self;
    let mut desugared = Desugared {
      track,
      named_objs,
      stmts: vec![],
      widget: None,
      errors: vec![],
      warnings: vec![],
    };
    let default_name = ribir_variable("ribir", widget.ty.span());
    let widget = widget.desugar(default_name, &mut desugared);
    desugared.widget = Some(widget);

    items
      .into_iter()
      .for_each(|item| item.desugar(&mut desugared));
    desugared
  }
}

impl DeclareWidget {
  fn desugar(self, default_name: Ident, desugared: &mut Desugared) -> WidgetNode {
    let Self { ty, fields: declare_fields, .. } = self;
    let mut id = None;
    let mut fields = smallvec![];
    let mut builtin_widgets: HashMap<_, SmallVec<[Field; 1]>, ahash::RandomState> = <_>::default();
    declare_fields
      .into_iter()
      .for_each(|f| match pick_id(f, &mut desugared.errors) {
        Ok(name) => id = Some(name),
        Err(DeclareField { member, expr, .. }) => {
          let value = FieldValue::Expr(expr.into());
          let field = Field { member, value };
          if let Some(ty) = FIELD_WIDGET_TYPE
            .get(field.member.to_string().as_str())
            .filter(|builtin_ty| !ty.is_ident(builtin_ty))
          {
            builtin_widgets.entry(*ty).or_default().push(field);
          } else {
            fields.push(field)
          }
        }
      });

    let parent = if let Some(name) = id {
      desugared.add_named_host_obj(DeclareObj::new(ty, name.clone(), fields));
      builtin_widgets.into_iter().for_each(|(ty, fields)| {
        let obj = builtin_obj(&name, ty, fields);
        desugared.add_named_builtin_obj(&name, obj);
      });
      ComposeItem::Id(name)
    } else {
      let mut objs = WIDGETS
        .iter()
        .rev()
        .filter_map(|b_widget| builtin_widgets.remove_entry(b_widget.ty))
        .map(|(ty, fields)| {
          let span = builtin_span(&default_name, &fields);
          let name = builtin_var_name(&default_name, span, ty);
          let ty = Ident::new(ty, name.span()).into();
          DeclareObj::new(ty, name, fields)
        })
        .collect::<SmallVec<_>>();
      assert!(builtin_widgets.is_empty());
      objs.push(DeclareObj::new(ty, default_name, fields));
      ComposeItem::ChainObjs(objs)
    };

    let children = self
      .children
      .into_iter()
      .enumerate()
      .map(|(idx, w)| {
        let name = child_variable(parent.name(), idx);
        w.desugar(name, desugared)
      })
      .collect();

    WidgetNode { parent, children }
  }
}

impl DeclareObj {
  pub fn new(ty: Path, name: Ident, fields: SmallVec<[Field; 1]>) -> Self {
    Self {
      ty,
      name,
      fields,
      stateful: false,
      desugar_from_on_event: false,
    }
  }
}

impl Item {
  fn desugar(self, desugared: &mut Desugared) {
    match self {
      Item::Transition(t) => {
        if let DesugaredObj::Obj(obj) = t.desugar(desugared) {
          let warning = DeclareWarning::DefObjWithoutId(obj.span().unwrap());
          desugared.warnings.push(warning)
        }
      }
      Item::Animate(a) => {
        if let DesugaredObj::Obj(obj) = a.desugar(desugared) {
          let warning = DeclareWarning::DefObjWithoutId(obj.span().unwrap());
          desugared.warnings.push(warning)
        }
      }
      Item::OnEvent(on_event) => on_event.desugar(desugared),
      Item::ModifyOn(modify_on) => {
        let (observe, subscribe_do) =
          desugared.desugar_quick_do(modify_on.observe, modify_on.quick_do);
        desugared
          .stmts
          .push(SubscribeItem::ObserveModifyDo { observe, subscribe_do });
      }
      Item::ChangeOn(change_on) => {
        let (observe, subscribe_do) =
          desugared.desugar_quick_do(change_on.observe, change_on.quick_do);
        desugared
          .stmts
          .push(SubscribeItem::ObserveChangeDo { observe, subscribe_do });
      }
    }
  }
}

enum DesugaredObj {
  Name(Ident),
  Obj(DeclareObj),
}

impl Transition {
  fn desugar(self, desugared: &mut Desugared) -> DesugaredObj {
    let Self { transition, fields, .. } = self;
    let ty = parse_quote_spanned! { transition.span => #transition <_>};
    let mut id = None;
    let fields = fields
      .into_iter()
      .filter_map(|f| match pick_id(f, &mut desugared.errors) {
        Ok(name) => {
          id = Some(name);
          None
        }
        Err(f) => Some(f.into()),
      })
      .collect();

    if let Some(name) = id {
      let c_name = name.clone();
      desugared.add_named_host_obj(DeclareObj::new(ty, name, fields));
      DesugaredObj::Name(c_name)
    } else {
      let name = ribir_variable("transition", transition.span());
      DesugaredObj::Obj(DeclareObj::new(ty, name, fields))
    }
  }
}

impl Animate {
  fn desugar(self, desugared: &mut Desugared) -> DesugaredObj {
    let Self {
      id,
      animate_token,
      from,
      transition,
      lerp_fn,
      ..
    } = self;

    let mut fields = smallvec![];
    if let Some(from) = from {
      let FromStateField { from, state, .. } = from;
      let value = FieldValue::Expr(TrackExpr::new(parse_quote! {#state}));
      fields.push(Field { member: from, value })
    }
    let lerp_fn = lerp_fn.map_or_else(
      || {
        let expr: Expr = parse_quote! {
          |from, to, rate| Lerp::lerp(from, to, rate)
        };
        Field {
          member: parse_quote! { lerp_fn },
          value: FieldValue::Expr(expr.into()),
        }
      },
      |f| f.into(),
    );
    fields.push(lerp_fn);
    if let Some(field) = transition.map(|t| t.desugar(desugared)) {
      fields.push(field);
    }

    let ty = parse_quote! {#animate_token<_, _, _, _, _, _>};
    if let Some(Id { name, .. }) = id {
      let c_name = name.clone();
      desugared.add_named_host_obj(DeclareObj::new(ty, name, fields));
      DesugaredObj::Name(c_name)
    } else {
      let name = ribir_variable("animate", animate_token.span());
      let mut obj = DeclareObj::new(ty, name, fields);
      obj.stateful = true;
      DesugaredObj::Obj(obj)
    }
  }
}

impl TransitionField {
  fn desugar(self, desugared: &mut Desugared) -> Field {
    let TransitionField { value, transition_kw: member, .. } = self;

    match value {
      AnimateTransitionValue::Transition(t) => match t.desugar(desugared) {
        DesugaredObj::Name(name) => {
          let value = FieldValue::Expr(TrackExpr::new(parse_quote! { #name.clone_stateful() }));
          Field { member, value }
        }
        DesugaredObj::Obj(obj) => Field {
          member,
          value: FieldValue::Obj(Box::new(obj)),
        },
      },
      AnimateTransitionValue::Expr(expr) => Field {
        member,
        value: FieldValue::Expr(expr.into()),
      },
    }
  }
}

impl ComposeItem {
  pub fn name(&self) -> &Ident {
    match self {
      ComposeItem::ChainObjs(objs) => &objs.last().expect("at least have one obj").name,
      ComposeItem::Id(name) => name,
    }
  }
}

impl Desugared {
  fn desugar_quick_do(&mut self, observe: Observe, quick_do: QuickDo) -> (TrackExpr, TrackExpr) {
    let desugar_animate_value = |mut animate: Animate, desugared: &mut Desugared| -> Expr {
      let mut init_state = quote! {};
      let animate_span = animate.span();
      if animate.from.is_none() {
        if let Some(path) = syn::parse2::<MemberPath>(quote! {#observe}).ok() {
          let from_value = ribir_variable("init_state", path.member.span());
          let c_from_value = ribir_suffix_variable(&from_value, "2");
          animate.from = Some(parse_quote_spanned! { path.span() =>
            from: State { #path: #from_value.borrow().clone()}
          });
          init_state = quote! {*#c_from_value.borrow_mut() = before.clone();};

          desugared.stmts.push(SubscribeItem::LetVar {
            name: from_value.clone(),
            value: TrackExpr::new(parse_quote_spanned! { animate_span =>
              std::rc::Rc::new(std::cell::RefCell::new(#path.clone()))
            }),
          });

          desugared.stmts.push(SubscribeItem::LetVar {
            name: c_from_value,
            value: TrackExpr::new(parse_quote_spanned! { animate_span =>
               #from_value.clone()
            }),
          });
        } else {
          let err = DeclareError::NoFromStateForAnimate(animate.span().unwrap());
          desugared.errors.push(err);
        }
      }

      match animate.desugar(desugared) {
        DesugaredObj::Name(name) => parse_quote_spanned! { animate_span => move |(before, after)| {
          if before != after {
            #init_state
            #name.run()
          }
        }},
        DesugaredObj::Obj(obj) => {
          let name = obj.name.clone();
          desugared.stmts.push(SubscribeItem::Obj(obj));
          parse_quote_spanned! { animate_span => move |(before, after)| {
            if before != after {
              #init_state
              #name.state_ref().run()
            }
          }}
        }
      }
    };

    let subscribe_do: Expr = match quick_do {
      super::parser::QuickDo::Flow(DataFlow { to, .. }) => {
        parse_quote_spanned! { to.span() => move |(_, after)| #to = after }
      }
      super::parser::QuickDo::Animate(a) => desugar_animate_value(a, self),
      super::parser::QuickDo::Transition(t) => {
        let animate: Animate = parse_quote_spanned! { t.span() =>
          Animate { transition: #t }
        };
        desugar_animate_value(animate, self)
      }
    };

    (
      TrackExpr::new(parse_quote_spanned! {observe.span() => #observe.clone()}),
      subscribe_do.into(),
    )
  }

  pub fn add_named_host_obj(&mut self, obj: DeclareObj) {
    if let Err(err) = self.named_objs.add_host_obj(obj) {
      self.errors.push(err);
    }
  }

  pub fn add_named_builtin_obj(&mut self, src_name: &Ident, obj: DeclareObj) {
    self.named_objs.add_builtin_obj(src_name, obj)
  }
}
impl OnEventDo {
  fn desugar(self, desugar: &mut Desugared) {
    let Self { observe, handlers, .. } = self;
    let Desugared { named_objs, stmts, errors, .. } = desugar;
    let observe_name = observe.get_ident();
    let mut listeners: HashMap<_, SmallVec<[Field; 1]>, ahash::RandomState> = <_>::default();
    for f in handlers {
      let member = &f.member;
      if member == MODIFY {
        stmts.push(SubscribeItem::ObserveModifyDo {
          observe: observe.clone().into_expr().into(),
          subscribe_do: f.expr.into(),
        })
      } else if member == CHANGE {
        stmts.push(SubscribeItem::ObserveChangeDo {
          observe: observe.clone().into_expr().into(),
          subscribe_do: f.expr.into(),
        })
      } else {
        if let Some(ty) = FIELD_WIDGET_TYPE.get(member.to_string().as_str()) {
          if is_listener(ty) {
            listeners.entry(ty).or_default().push(f.into());
            continue;
          }
        }
        errors.push(DeclareError::OnInvalidField(member.clone()));
      }
    }

    if listeners.is_empty() {
      return;
    }

    if let Some(name) = observe_name {
      if !named_objs.contains(&name) {
        errors.push(DeclareError::EventObserveOnUndeclared(name.clone()));
      }
      listeners.into_iter().for_each(|(ty, fields)| {
        let mut obj = builtin_obj(name, ty, fields);
        obj.desugar_from_on_event = true;
        desugar.add_named_builtin_obj(name, obj);
      });
    } else {
      errors.push(DeclareError::OnInvalidTarget(observe.span().unwrap()));
    }
  }
}

impl From<DeclareField> for Field {
  fn from(f: DeclareField) -> Self {
    Self {
      member: f.member,
      value: FieldValue::Expr(f.expr.into()),
    }
  }
}

impl From<Expr> for TrackExpr {
  fn from(expr: Expr) -> Self {
    TrackExpr {
      expr: expr,
      used_name_info: <_>::default(),
    }
  }
}

fn pick_id(f: DeclareField, errors: &mut Vec<DeclareError>) -> Result<Ident, DeclareField> {
  let DeclareField { member, expr, .. } = &f;
  if member == ID {
    let name = syn::parse2::<Ident>(quote! {#expr});
    name.map_err(move |err| {
      errors.push(DeclareError::SynErr(err));
      f
    })
  } else {
    Err(f)
  }
}

impl Observe {
  fn get_ident(&self) -> Option<&Ident> {
    match self {
      Observe::Name(n) => Some(n),
      Observe::Expr(Expr::Path(p)) => p.path.get_ident(),
      _ => None,
    }
  }

  fn into_expr(self) -> Expr {
    match self {
      Observe::Name(n) => Expr::Path(ExprPath {
        attrs: vec![],
        qself: None,
        path: n.into(),
      }),
      Observe::Expr(e) => e,
    }
  }
}

impl ToTokens for Observe {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    match self {
      Observe::Name(n) => n.to_tokens(tokens),
      Observe::Expr(e) => e.to_tokens(tokens),
    }
  }
}

impl NamedObjMap {
  pub fn get(&self, name: &Ident) -> Option<&NamedObj> { self.0.get(name) }

  pub fn contains(&self, name: &Ident) -> bool { self.0.contains_key(name) }

  pub fn get_mut(&mut self, name: &Ident) -> Option<&mut NamedObj> { self.0.get_mut(name) }

  pub fn names(&self) -> impl Iterator<Item = &Ident> { self.0.keys() }

  pub fn objs(&self) -> impl Iterator<Item = &NamedObj> { self.0.values() }

  pub fn objs_mut(&mut self) -> impl Iterator<Item = &mut NamedObj> { self.0.values_mut() }

  pub fn iter(&self) -> impl Iterator<Item = (&Ident, &NamedObj)> { self.0.iter() }

  pub fn get_name_obj(&self, name: &Ident) -> Option<(&Ident, &NamedObj)> {
    self.0.get_key_value(name)
  }

  fn add_host_obj(&mut self, obj: DeclareObj) -> Result<(), DeclareError> {
    if let Some((name, _)) = self.0.get_key_value(&obj.name) {
      let err = DeclareError::DuplicateID([name.clone(), obj.name.clone()]);
      Err(err)
    } else {
      self.0.insert(obj.name.clone(), NamedObj::Host(obj));
      Ok(())
    }
  }

  fn add_builtin_obj(&mut self, src_name: &Ident, obj: DeclareObj) {
    match self.0.get_mut(&obj.name) {
      Some(NamedObj::Host(_)) => unreachable!("named object conflict with listener name."),
      Some(NamedObj::Builtin { obj: o, src_name }) => {
        let name = obj.name.clone();
        let n = NamedObj::DuplicateListener {
          src_name: src_name.clone(),
          objs: vec![o.clone(), obj],
        };
        self.0.insert(name, n);
      }
      Some(NamedObj::DuplicateListener { objs, .. }) => objs.push(obj),
      None => {
        let src_name = src_name.clone();
        self
          .0
          .insert(obj.name.clone(), NamedObj::Builtin { src_name, obj });
      }
    }
  }
}

fn builtin_span(host: &Ident, fields: &SmallVec<[Field; 1]>) -> Span {
  if fields.is_empty() {
    host.span()
  } else {
    let span = fields[0].member.span();
    fields[1..]
      .iter()
      .fold(span, |span, f| span.join(f.member.span()).unwrap())
  }
}

pub fn builtin_obj(src_name: &Ident, ty: &str, fields: SmallVec<[Field; 1]>) -> DeclareObj {
  let span = builtin_span(src_name, &fields);
  let name = builtin_var_name(&src_name, span, ty);
  let ty = Ident::new(ty, src_name.span()).into();
  DeclareObj::new(ty, name, fields)
}
