use crate::prelude::*;
use std::{cell::Cell, rc::Rc};
use winit::window::CursorIcon;

/// `Cursor` is an attribute to assign an `cursor` to a widget.
#[derive(Debug)]
pub struct Cursor(Rc<Cell<CursorIcon>>);

#[derive(Declare)]
pub struct CursorDeclarer {
  cursor: CursorIcon,
}

impl IntoWidget for CursorDeclarer {
  type W = Cursor;

  #[inline]
  fn into_widget(self) -> Self::W { Cursor(Rc::new(Cell::new(self.cursor))) }
}

impl SingleChildWidget for Cursor {
  fn have_child<C: IntoOptionChild<M>, M>(self, child: C) -> SingleChild<Self> { todo!() }
}

// pub fn cursor_attach<W>(icon: CursorIcon, w: W) -> W::Target
// where
//   W: AttachAttr,
// {
//   let mut default = false;
//   let w = w.inspect_or_else(
//     || {
//       default = true;
//       Cursor::new(icon)
//     },
//     |attr| attr.set_icon(icon),
//   );
//   if !default {
//     return w;
//   }

//   w.on_pointer_move(move |e| {
//     let mut ctx = e.context();
//     if e.point_type == PointerType::Mouse
//       && e.buttons == MouseButtons::empty()
//       && ctx.updated_cursor().is_none()
//     {
//       if let Some(icon) = ctx.find_attr::<Cursor>().map(|c| c.0.get()) {
//         ctx.set_cursor(icon);
//       }
//     }
//   })
// }

impl Cursor {
  pub fn new(icon: CursorIcon) -> Self { Cursor(Rc::new(Cell::new(icon))) }

  #[inline]
  pub fn icon(&self) -> CursorIcon { self.0.get() }

  #[inline]
  pub fn set_icon(&self, icon: CursorIcon) { self.0.set(icon) }
}

impl Default for Cursor {
  #[inline]
  fn default() -> Self { Self::new(CursorIcon::Default) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use winit::event::{DeviceId, WindowEvent};

  #[test]
  fn tree_down_up() {
    struct RowTree;
    impl Compose for RowTree {
      fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        widget! {
          declare SizedBox {
            size: Size::new(f32::INFINITY, f32::INFINITY),
            cursor: CursorIcon::AllScroll,
            Row{
              v_align: CrossAxisAlign::Start,
              h_align: MainAxisAlign::Start,
              SizedBox {
                size: Size::new(200., 200.),
                cursor: CursorIcon::Hand,
                Row {
                  v_align: CrossAxisAlign::Start,
                  h_align: MainAxisAlign::Start,
                  SizedBox {
                    size:  Size::new(100., 100.),
                    cursor: CursorIcon::Help,
                  }
                }
              }
            }
          }
        }
      }
    }

    let mut wnd = Window::without_render(RowTree.box_it(), Size::new(400., 400.));

    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    let ctx = &mut wnd.context;
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::Help));

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (101f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::Hand));

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (201f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::AllScroll));

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (101f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::Hand));

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::Help));
  }
}
