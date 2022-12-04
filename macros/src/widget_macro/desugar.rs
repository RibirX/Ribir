use proc_macro2::Span;
use quote::quote;
use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;
use syn::{
  parse_quote, parse_quote_spanned, spanned::Spanned, token::Brace, Block, Expr, Ident, Path, Stmt,
};

use super::{
  child_variable, guard_ident,
  parser::{DeclareField, DeclareSingle, DeclareWidget, Item, MacroSyntax, States, TransProps},
  ribir_variable, ScopeUsedInfo, TrackExpr, WIDGETS, WIDGET_OF_BUILTIN_FIELD,
};
use crate::{
  error::{DeclareError, DeclareWarning},
  widget_macro::builtin_var_name,
};

#[derive(Default)]
pub struct NamedObjMap(HashMap<Ident, NamedObj, ahash::RandomState>);

pub const ID: &str = "id";
pub struct Desugared {
  pub init: Option<InitStmts>,
  pub states: Option<States>,
  pub named_objs: NamedObjMap,
  pub widget: Option<WidgetNode>,
  pub finally: Option<FinallyBlock>,
  pub errors: Vec<DeclareError>,
  pub warnings: Vec<DeclareWarning>,
}

pub struct InitStmts {
  pub stmts: Vec<Stmt>,
  pub used_name_info: ScopeUsedInfo,
}

#[derive(Default)]
pub struct FinallyBlock {
  pub brace_token: Brace,
  pub stmts: Vec<FinallyStmt>,
  pub used_name_info: ScopeUsedInfo,
}

pub enum FinallyStmt {
  Stmt(Stmt),
  Obj(DeclareObj),
}

#[derive(Debug, Clone)]
pub struct DeclareObj {
  pub ty: Path,
  pub name: Ident,
  pub fields: SmallVec<[Field; 1]>,
  pub stateful: bool,
  pub watch_stmts: SmallVec<[Stmt; 1]>,
}

#[derive(Debug)]
pub struct BuiltinObj {
  pub obj: DeclareObj,
  pub src_name: Option<Ident>,
}

#[derive(Debug)]
pub enum NamedObj {
  Host(DeclareObj),
  Builtin { src_name: Ident, obj: DeclareObj },
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
  pub node: ComposeItem,
  pub children: Vec<WidgetNode>,
}

pub enum ComposeItem {
  ChainObjs(SmallVec<[DeclareObj; 1]>),
  Id(Ident),
}

impl MacroSyntax {
  pub fn desugar(self) -> Desugared {
    let named_objs = NamedObjMap::default();
    let MacroSyntax { init, states, widget, items, finally } = self;

    let mut desugared = Desugared {
      init: init.map(|init| InitStmts {
        stmts: init.block.stmts,
        used_name_info: <_>::default(),
      }),
      states,
      named_objs,
      widget: None,
      finally: finally.map(|f| {
        let Block { brace_token, stmts } = f.block;
        FinallyBlock {
          brace_token,
          stmts: stmts.into_iter().map(|s| FinallyStmt::Stmt(s)).collect(),
          used_name_info: <_>::default(),
        }
      }),
      errors: vec![],
      warnings: vec![],
    };
    let default_name = ribir_variable("ribir", widget.ty_path().span());
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
    match self {
      DeclareWidget::Literal {
        ty, fields: declare_fields, children, ..
      } => {
        let mut id = None;
        let mut fields = smallvec![];
        let mut builtin_widgets: HashMap<_, SmallVec<[Field; 1]>, ahash::RandomState> =
          <_>::default();
        declare_fields
          .into_iter()
          .for_each(|f| match pick_id(f, &mut desugared.errors) {
            Ok(name) => id = Some(name),
            Err(DeclareField { member, expr, .. }) => {
              let value = FieldValue::Expr(expr.into());
              let field = Field { member, value };
              if let Some(ty) = WIDGET_OF_BUILTIN_FIELD
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
            desugared.add_named_builtin_obj(name.clone(), obj);
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

        let children = children
          .into_iter()
          .enumerate()
          .map(|(idx, w)| {
            let mut name = child_variable(parent.name(), idx);
            name.set_span(w.ty_path().span());
            w.desugar(name, desugared)
          })
          .collect();

        WidgetNode { node: parent, children }
      }
      DeclareWidget::Call(call) => {
        let expr: Expr = parse_quote!(#call);
        expr_as_widget_node(expr, default_name)
      }
      DeclareWidget::Path(path) => {
        let expr: Expr = parse_quote!(#path);
        expr_as_widget_node(expr, default_name)
      }
    }
  }
}

fn expr_as_widget_node(expr: Expr, default_name: Ident) -> WidgetNode {
  let ty = parse_quote_spanned!(expr.span() => DynWidget);
  let field = Field {
    member: parse_quote_spanned!(expr.span() => dyns),
    value: FieldValue::Expr(expr.into()),
  };
  let obj = DeclareObj::new(ty, default_name, smallvec![field]);
  WidgetNode {
    node: ComposeItem::ChainObjs(smallvec![obj]),
    children: vec![],
  }
}
impl DeclareObj {
  pub fn new(ty: Path, name: Ident, fields: SmallVec<[Field; 1]>) -> Self {
    Self {
      ty,
      name,
      fields,
      stateful: false,
      watch_stmts: <_>::default(),
    }
  }
}

impl Item {
  fn desugar(self, desugared: &mut Desugared) {
    match self {
      Item::TransProps(TransProps { transition, props, fields, .. }) => {
        let by = fields.iter().enumerate().find(|(_, f)| f.member == "by");
        let transition = if let Some(by) = by {
          if fields.len() > 1 {
            desugared
              .errors
              .push(DeclareError::TransitionByConflict(by.1.span().unwrap()));
            return;
          }
          FieldValue::Expr(by.1.expr.clone().into())
        } else {
          let fields = fields
            .iter()
            .map(|f| Field {
              member: f.member.clone(),
              value: FieldValue::Expr(f.expr.clone().into()),
            })
            .collect();
          let name = ribir_variable("transition", transition.span());
          let mut obj = DeclareObj::new(parse_quote!(Transition), name, fields);
          obj.stateful = true;
          FieldValue::Obj(Box::new(obj))
        };

        let stmts = &mut desugared
          .finally
          .get_or_insert_with(FinallyBlock::default)
          .stmts;

        props.into_iter().for_each(|p| {
          let span = p.span();
          let prop = ribir_variable("prop", span);
          stmts.push(FinallyStmt::Stmt(
            parse_quote_spanned! { span => let #prop = #p;},
          ));

          let prop_changes = ribir_variable("prop_changes", span);
          stmts.push(FinallyStmt::Stmt(
            parse_quote_spanned! { span => let #prop_changes = #prop.changes();},
          ));
          let from = ribir_variable("from", span);
          stmts.push(FinallyStmt::Stmt(
            parse_quote_spanned! { span => let #from = #prop.get();},
          ));
          let name = ribir_variable("animate", transition.span());
          let transition = Field {
            member: Ident::new("transition", span),
            value: transition.clone(),
          };
          let from_value: Expr = parse_quote!(#from);
          let from = Field {
            member: Ident::new("from", span),
            value: FieldValue::Expr(from_value.into()),
          };
          let prop_value: Expr = parse_quote!(#prop);
          let prop = Field {
            member: Ident::new("prop", span),
            value: FieldValue::Expr(prop_value.into()),
          };

          let mut obj = DeclareObj::new(
            parse_quote_spanned!(span => Animate),
            name.clone(),
            smallvec![transition, prop, from],
          );

          obj.stateful = true;
          stmts.push(FinallyStmt::Obj(obj));
          let guard = guard_ident(span);
          stmts.push(FinallyStmt::Stmt(parse_quote_spanned! { span =>
            let #guard = #prop_changes.subscribe(move |(old, _)| {
              #name.state_ref().from = old;
              #name.state_ref().run();
            })
            .unsubscribe_when_dropped();
          }));
          stmts.push(FinallyStmt::Stmt(
            parse_quote_spanned! { span => move_to_widget!(#guard); },
          ));
        });
      }
      Item::Transition(d) | Item::Animate(d) => {
        if let DesugaredObj::Obj(obj) = d.desugar(desugared) {
          let warning = DeclareWarning::DefObjWithoutId(obj.span().unwrap());
          desugared.warnings.push(warning)
        }
      }
    }
  }
}

enum DesugaredObj {
  Name(Ident),
  Obj(DeclareObj),
}

impl DeclareSingle {
  fn desugar(self, desugared: &mut Desugared) -> DesugaredObj {
    let Self { ty, fields, .. } = self;
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
      let name = ribir_variable("obj", ty.span());
      DesugaredObj::Obj(DeclareObj::new(ty, name, fields))
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
  pub fn add_named_host_obj(&mut self, obj: DeclareObj) {
    if let Err(err) = self.named_objs.add_host_obj(obj) {
      self.errors.push(err);
    }
  }

  pub fn add_named_builtin_obj(&mut self, src_name: Ident, obj: DeclareObj) {
    self.named_objs.add_builtin_obj(src_name, obj)
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

  fn add_builtin_obj(&mut self, src_name: Ident, obj: DeclareObj) {
    let v = self
      .0
      .insert(obj.name.clone(), NamedObj::Builtin { src_name, obj });

    assert!(v.is_none(), "builtin widget already have");
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

impl NamedObj {
  pub fn name(&self) -> &Ident {
    match self {
      NamedObj::Host(obj) => &obj.name,
      NamedObj::Builtin { obj, .. } => &obj.name,
    }
  }

  pub fn ty(&self) -> &Path {
    match self {
      NamedObj::Host(obj) => &obj.ty,
      NamedObj::Builtin { obj, .. } => &obj.ty,
    }
  }
}
