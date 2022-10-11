use crate::widget_macro::NameUsedInfo;
use proc_macro::{Diagnostic, Level, Span};
use proc_macro2::TokenStream;

use quote::ToTokens;
use syn::Ident;

#[derive(Debug)]
pub struct CircleUsedPath {
  pub obj: Ident,
  pub member: Option<Ident>,
  pub used_widget: Ident,
  pub used_info: NameUsedInfo,
}

#[derive(Debug)]
pub enum DeclareError {
  DuplicateID([Ident; 2]),
  CircleDepends(Box<[CircleUsedPath]>),
  ExprWidgetInvalidField(Vec<Span>),
  OnInvalidTarget(Span),
  OnInvalidField(Ident),
  NoFromStateForAnimate(Span),
  EventObserveOnUndeclared(Ident),
  DependsOnDupListener {
    declare_at: Vec<Span>,
    used_at: Vec<Span>,
  },
  SynErr(syn::Error),
}

#[derive(Debug)]
pub enum DeclareWarning {
  UnusedName(Span),
  ObserveIsConst(Span),
  DefObjWithoutId(Span),
}

impl DeclareError {
  pub fn into_compile_error(&self, tokens: &mut TokenStream) {
    let mut diagnostic = Diagnostic::new(Level::Error, "");
    match self {
      DeclareError::DuplicateID([id1, id2]) => {
        assert_eq!(id1, id2);
        diagnostic.set_spans(vec![id1.span().unwrap(), id2.span().unwrap()]);
        diagnostic.set_message(format!(
          "Same `id: {}` assign to multiple objects, id must be unique.",
          id1
        ));
      }
      DeclareError::CircleDepends(path) => {
        let (msg, spans, note_spans) = path_info(&path);
        let msg = format!(
          "There is a directly circle depends exist, this will cause infinite loop: {}",
          msg
        );
        diagnostic.set_spans(spans);
        diagnostic.set_message(msg);
        let note_msg = "You can use `change` event to break circle, which \
          will trigger only if the value really changed by compare if the value \
          equal before modify and after.";
        diagnostic = diagnostic.span_note(note_spans, note_msg);
      }
      DeclareError::ExprWidgetInvalidField(spans) => {
        diagnostic.set_spans(spans.clone());
        diagnostic.set_message("`ExprWidget` only accept `expr` field.");
      }
      DeclareError::OnInvalidTarget(span) => {
        diagnostic.set_spans(*span);
        diagnostic.set_message(
          "only the id of widget declared in `widget!` can used as the target of `on` group",
        );
      }
      DeclareError::OnInvalidField(f) => {
        diagnostic.set_spans(f.span().unwrap());
        diagnostic.set_message(&format!(
          "`{f}` is not allow use in `on` group, only listeners support.",
        ));
      }
      DeclareError::NoFromStateForAnimate(_) => todo!(),
      DeclareError::EventObserveOnUndeclared(name) => {
        diagnostic.set_spans(name.span().unwrap());
        diagnostic.set_message(&format!(
          "Not found `{name}` declare in the `widget!`, `on` item only \
          allow to observe widget declared in the `widget!` macro which \
          itself located in.",
        ));
      }
      DeclareError::SynErr(err) => err.clone().into_compile_error().to_tokens(tokens),
      DeclareError::DependsOnDupListener { declare_at, used_at } => {
        diagnostic.set_spans(used_at.clone());
        diagnostic.set_message(&format!(
          "Object can't be depends which have many instance.",
        ));
        diagnostic = diagnostic.span_help(declare_at.clone(), "declare at here.");
      }
    };

    diagnostic.emit();
  }
}

// return a tuple compose by the string display of path, the path follow spans
// and the spans of where `#[skip_nc]` can be added.
fn path_info(path: &[CircleUsedPath]) -> (String, Vec<Span>, Vec<Span>) {
  let msg = path
    .iter()
    .map(|info| {
      if let Some(m) = info.member.as_ref() {
        format!("{}.{} ~> {} ", info.obj, m, info.used_widget)
      } else {
        format!("{} ~> {} ", info.obj, info.used_widget)
      }
    })
    .collect::<Vec<_>>()
    .join(", ");

  let spans = path.iter().fold(vec![], |mut res, info| {
    res.push(info.obj.span().unwrap());
    if let Some(m) = info.member.as_ref() {
      res.push(m.span().unwrap());
    }

    res.push(info.used_widget.span().unwrap());
    let t_spans = info.used_info.spans.iter().map(|s| s.unwrap());
    res.extend(t_spans);
    res
  });

  let note_spans = path
    .iter()
    .map(|info| {
      if let Some(m) = info.member.as_ref() {
        m.span().unwrap()
      } else {
        info
          .used_info
          .spans
          .iter()
          .fold(info.obj.span(), |s1, s2| s2.join(s1).unwrap())
          .unwrap()
      }
    })
    .collect::<Vec<_>>();

  (msg, spans, note_spans)
}

impl DeclareWarning {
  pub fn emit_warning(&self) {
    let mut d = Diagnostic::new(Level::Warning, "");
    match self {
      DeclareWarning::UnusedName(span) => {
        d.set_spans(*span);
        d.set_message(format!("assigned id but not be used in anywhere."));
        d = d.span_help(*span, "Remove this line.");
      }
      DeclareWarning::ObserveIsConst(span) => {
        d.set_spans(*span);
        d.set_message("Observe a expr but not depends on anything, this will do nothing.");
        d = d.help("Try to remove it.");
      }
      DeclareWarning::DefObjWithoutId(span) => {
        d.set_spans(*span);
        d.set_message("Def an object without id.");
        d = d.help("Try to assign an `id` for it.");
      }
    };
    d.emit();
  }
}
