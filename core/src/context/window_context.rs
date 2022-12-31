use std::{cell::RefCell, rc::Rc, time::Instant};

use ribir_painter::TypographyStore;
use rxrust::prelude::LocalObservable;

use super::AppContext;
use crate::{
  builtin_widgets::Theme,
  events::focus_mgr::{FocusManager, FocusType, FocustHandle},
  ticker::{FrameMsg, FrameTicker},
  widget::{TreeArena, WidgetId},
};

#[derive(Clone)]
pub struct WindowCtx {
  pub(crate) frame_ticker: FrameTicker,
  pub(crate) focus_mgr: Rc<RefCell<FocusManager>>,
  pub(crate) app_ctx: AppContext,
}

impl WindowCtx {
  pub fn app_theme(&self) -> Rc<Theme> { self.app_ctx.app_theme.clone() }

  pub fn new(app_ctx: AppContext) -> Self {
    Self {
      app_ctx,
      focus_mgr: Rc::new(RefCell::new(FocusManager::default())),
      frame_ticker: FrameTicker::default(),
    }
  }

  pub fn typography_store(&self) -> &TypographyStore { &self.app_ctx.typography_store }

  pub fn frame_tick_stream(&self) -> impl LocalObservable<'static, Item = FrameMsg, Err = ()> {
    self.frame_ticker.frame_tick_stream()
  }

  pub(crate) fn begin_frame(&mut self) {
    self.frame_ticker.emit(FrameMsg::NewFrame(Instant::now()));
  }
  pub(crate) fn layout_ready(&mut self) {
    self
      .frame_ticker
      .emit(FrameMsg::LayoutReady(Instant::now()));
  }

  pub(crate) fn end_frame(&mut self) {
    self.app_ctx.end_frame();
    self.frame_ticker.emit(FrameMsg::Finish(Instant::now()));
  }

  pub(crate) fn next_focus(&self, arena: &TreeArena) {
    self.focus_mgr.borrow_mut().next_focus(arena);
  }

  pub(crate) fn prev_focus(&self, arena: &TreeArena) {
    self.focus_mgr.borrow_mut().prev_focus(arena);
  }

  pub(crate) fn focus_handle(&self, wid: WidgetId) -> FocustHandle {
    FocusManager::focus_handle(&self.focus_mgr, wid)
  }

  pub(crate) fn add_focus_node(
    &self,
    wid: WidgetId,
    auto_focus: bool,
    focus_tyep: FocusType,
    arena: &TreeArena,
  ) {
    self
      .focus_mgr
      .borrow_mut()
      .add_focus_node(wid, auto_focus, focus_tyep, arena);
  }

  pub(crate) fn remove_focus_node(&self, wid: WidgetId, focus_tyep: FocusType) {
    self
      .focus_mgr
      .borrow_mut()
      .remove_focus_node(wid, focus_tyep);
  }
}
