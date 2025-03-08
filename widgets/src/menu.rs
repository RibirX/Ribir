use std::cell::RefCell;

use ribir_core::prelude::*;
use smallvec::smallvec;

use crate::prelude::*;

class_names! {
  #[doc = "class name for the Menu"]
  MENU,
  #[doc="class name for Menu Item in unselected state"]
  MENU_ITEM,
  #[doc="class name for Menu Item in disabled state"]
  MENU_ITEM_DISABLED,
  #[doc="class name for Menu Item in selected state"]
  MENU_ITEM_SELECTED,
  #[doc="class name for MenuDivider"]
  MENU_DIVIDER,
  #[doc="class name for Menu label"]
  MENU_ITEM_LABEL,
  #[doc="class name for Menu hint label"]
  MENU_ITEM_HINT_TEXT,
  #[doc="class name for Menu leading icon"]
  MENU_ITEM_LEADING,
  #[doc="class name for Menu trailing icon"]
  MENU_ITEM_TRAILING,
}

pub enum MenuEventData {
  /// Emitted when the menu's selected is changed
  Select(bool, MenuItemControl),
  /// Emitted when the menu item is entered, the MenuItemControl is the item
  /// that is triggered
  Enter(MenuItemControl),
  /// Emitted when the menu is completed,
  /// the MenuItemControl is the item that is triggered,
  /// the Option<Sc<dyn Any>> is the data that is returned from the item.
  /// if the sub_menu's complete event is not stopped, the menu will be closed.
  Complete(MenuItemControl, Option<Sc<dyn Any>>),
}

/// the menu event will be emitted from the menu item that is triggered
pub type MenuEvent = CustomEvent<MenuEventData>;

/// Menu, must be use within the MenuControl.
///
/// You can typically use the built-in [`MenuItem`] to create a menu. And  use
/// [`MenuDivider`] to create a divider in the menu.
/// Also you can use any widget as item of Menu, and the item will be wrap with
/// Provider<MenuItemControl>, which can be used to interact with the menu.
///
/// # Example
/// ```rust no_run
/// # use ribir::prelude::*;
/// let w = fn_widget! {
///   let sub_menu = MenuControl::new(menu! {
///     @MenuItem {
///       @ Leading::new( @Icon { @ { svgs::MENU } })
///       @ { "sub_menu" }
///     }
///   });
///   let menu = MenuControl::new(menu! {
///     @MenuItem {
///       @ Leading::new( @Icon { @ { svgs::MENU } })
///       @ { "Menu Item1" }
///       @ { sub_menu.clone() }
///     }
///     @MenuDivider {}
///     @MenuItem { @ { "Menu Item2" } }
///   });
///   @Container {
///     size: Size::new(f32::INFINITY, f32::INFINITY),
///     on_tap: move |e| {
///       menu.show_at(e.position(), None, &e.window());
///     },
///   }
/// };
/// App::run(w);
/// ```
#[derive(Declare)]
pub struct Menu {}

/// the controller of the popup menu
#[derive(Clone, ChildOfCompose)]
pub struct MenuControl(Sc<RefCell<MenuData>>);

struct MenuItemData {
  wid: TrackId,
  disabled: bool,
}

struct MenuData {
  id: Option<TrackId>,
  handle: Option<Overlay>,
  item_trigger: Option<MenuItemControl>,
  selected: Option<usize>,
  items: Vec<MenuItemData>,
  gen: GenWidget,
}

impl MenuControl {
  /// Receive a function generator of widget return a MenuControl
  pub fn new(gen: impl Into<GenWidget>) -> Self {
    Self(Sc::new(RefCell::new(MenuData {
      gen: gen.into(),
      handle: None,
      item_trigger: None,
      selected: None,
      items: vec![],
      id: None,
    })))
  }

  /// Check if the menu is showing
  pub fn is_show(&self) -> bool { self.0.borrow().handle.is_some() }

  /// Show the menu
  pub fn show(&self, parent: Option<MenuItemControl>, wnd: &Sc<Window>) {
    let gen = self.0.borrow().gen.clone();
    self.inner_show(gen, parent, wnd);
  }

  /// Focus the menu
  pub fn focus(&self, wnd: &Sc<Window>) {
    if let Some(id) = self
      .0
      .borrow()
      .id
      .as_ref()
      .and_then(|id| id.get())
    {
      wnd.request_focus(id);
    }
  }

  /// Show the menu around the target rect, the target rect is relative to the
  /// window
  pub fn show_around(&self, target: Rect, parent: Option<MenuItemControl>, wnd: &Sc<Window>) {
    self.show_map(anchor_around(target), parent, wnd);
  }

  /// Show the menu around the global position
  pub fn show_at(&self, pos: Point, parent: Option<MenuItemControl>, wnd: &Sc<Window>) {
    self.show_map(anchor_around(Rect::new(pos, Size::zero())), parent, wnd);
  }

  pub fn show_map<F>(&self, mut f: F, parent: Option<MenuItemControl>, wnd: &Sc<Window>)
  where
    F: FnMut(Widget<'static>) -> Widget<'static> + 'static,
  {
    let gen = self.0.borrow().gen.clone();
    let gen = move || f(gen.gen_widget());
    self.inner_show(gen.into(), parent, wnd);
  }

  /// Close the menu
  pub fn close(&self, wnd: &Sc<Window>) {
    let mut this = self.0.borrow_mut();
    if this.handle.is_none() {
      return;
    }

    if let Some(parent) = this.item_trigger.take() {
      parent.menu.focus(wnd);
    }

    if let Some(handle) = this.handle.as_ref() {
      handle.close();
    }

    this.items.clear();
    this.handle = None;
    this.item_trigger = None;
    this.selected = None;
    this.id = None;
  }

  /// Select the next selectable item
  pub fn select_next(&self, forward: bool, wnd: &Sc<Window>) {
    let calc_next_idx = |this: &MenuData| {
      let len = this.items.len();
      if len == 0 {
        return None;
      }
      let old_index = this.selected;
      let (offset, start_idx) = if forward { (1, len - 1) } else { (len - 1, 0) };
      let mut idx = if let Some(idx) = old_index { idx } else { start_idx };
      for _ in 0..len {
        idx = (idx + offset) % len;
        if !this.items[idx].disabled {
          return Some(idx);
        }
      }
      None
    };
    let idx = calc_next_idx(&self.0.borrow());
    self.select(idx, wnd);
  }

  /// Select the nth Selectable item(not disabled).
  ///
  /// Parameters:
  ///
  /// nth: if the nth Some(idx) specified, select the idx selectable item, else
  /// no item will be selected in the menu.
  ///
  /// wnd: the window
  ///
  /// Returns:
  /// return true if select successfully, false otherwise.
  fn select(&self, idx: Option<usize>, wnd: &Sc<Window>) -> bool {
    let mut this = self.0.borrow_mut();
    if this.selected == idx {
      return true;
    }
    if let Some(selected) = idx {
      if this.items[selected].disabled {
        return false;
      }
    }

    if let Some(selected) = this.selected {
      if let Some(from) = this.items[selected].wid.get() {
        wnd.bubble_custom_event(
          from,
          MenuEventData::Select(false, MenuItemControl { idx: selected, menu: self.clone() }),
        );
      }
    }

    if let Some(selected) = idx {
      if let Some(from) = this.items[selected].wid.get() {
        wnd.bubble_custom_event(
          from,
          MenuEventData::Select(true, MenuItemControl { idx: selected, menu: self.clone() }),
        );
      }
    }

    this.selected = idx;
    true
  }

  /// Enter the item, emit the MenuEventData::Enter.
  ///
  /// Parameters:
  ///
  /// nth: specified the idx item,
  ///
  /// wnd: the window
  ///
  /// Return:
  /// return true if enter successfully, false otherwise.
  pub fn enter(&self, idx: usize, wnd: &Sc<Window>) -> bool {
    if !self.select(Some(idx), wnd) {
      return false;
    }

    let wid = self.0.borrow().items[idx].wid.get();
    if let Some(from) = wid {
      wnd.bubble_custom_event(
        from,
        MenuEventData::Enter(MenuItemControl { idx, menu: self.clone() }),
      );
    }
    true
  }

  fn selected(&self) -> Option<usize> { self.0.borrow().selected }

  fn inner_show(&self, gen: GenWidget, parent: Option<MenuItemControl>, wnd: &Sc<Window>) {
    let handle = self.clone();
    let fn_gen = fn_widget! {
      let mut w = FatObj::new(gen.clone());
      handle.0.borrow_mut().id = Some($w.track_id());
      @Providers {
        providers: smallvec![Provider::new(handle.clone())],
        @ $w{
          on_custom_concrete_event: move |e: &mut MenuEvent| {
            if let MenuEventData::Complete(item, data) = e.data() {
              if let Some(item) = item.parent() {
                item.bubble(MenuEventData::Complete(item.clone(), data.clone()), &e.window());
              }
              item.close(&e.window());
            }
          },
        }
      }
    };

    let style = if parent.is_some() {
      OverlayStyle { auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE, mask: None }
    } else {
      OverlayStyle { auto_close_policy: AutoClosePolicy::TAP_OUTSIDE, mask: None }
    };

    let handle = Overlay::new(fn_gen, style);
    handle.show(wnd.clone());
    let mut this = self.0.borrow_mut();
    this.item_trigger = parent.clone();
    this.handle = Some(handle);
  }

  fn new_item(&self, wid: TrackId) -> MenuItemControl {
    self
      .0
      .borrow_mut()
      .items
      .push(MenuItemData { wid, disabled: false });
    let idx = self.0.borrow().items.len() - 1;
    MenuItemControl { menu: self.clone(), idx }
  }
}

fn anchor_around(target: Rect) -> impl FnMut(Widget<'static>) -> Widget<'static> {
  move |w: Widget<'static>| -> Widget<'static> {
    fn_widget! {
      let w = FatObj::new(w);
      @ $w {
        global_anchor_x: GlobalAnchorX::custom(move |host, wnd| {
          let host_id = host.get().unwrap();
          let wnd_size = wnd.size();
          if let Some(size) = wnd.widget_size(host_id) {
            if target.max_x() + size.width < wnd_size.width {
              return Ok(target.max_x())
            } else {
              return Ok((0_f32).max(target.min_x() - size.width))
            }
          }
          Ok(0.)
        }),
        global_anchor_y: GlobalAnchorY::custom(move |host, wnd| {
          let host_id = host.get().unwrap();
          let wnd_size = wnd.size();
          if let Some(size) = wnd.widget_size(host_id) {
            if target.min_y() + size.height < wnd_size.height {
              return Ok(target.min_y())
            } else {
              return Ok((0_f32).max(wnd_size.height - size.height))
            }
          }
          Ok(0.)
        })
      }
    }
    .into_widget()
  }
}

#[derive(ChildOfCompose)]
pub struct MenuHintText(TextInit);
impl MenuHintText {
  pub fn new<const M: usize>(child: impl IntoChildCompose<TextInit, M>) -> Self {
    MenuHintText(child.into_child_compose())
  }
}

#[derive(Template)]
pub struct MenuItemChild {
  /// label text
  label: TextInit,
  /// trailing hint text
  trailing_text: Option<MenuHintText>,
  /// leading icon
  leading: Option<Leading<Widget<'static>>>,
  /// trailing icon
  trailing: Option<Trailing<Widget<'static>>>,
  /// sub menu
  sub_menu: Option<MenuControl>,
}

/// MenuItem, which can be used in [`Menu`].
///
/// MenuItem receives MenuItemChild as child, including a label text, a leading
/// icon, a trailing icon, and a sub menu. when the item is hovered or entered,
/// the sub menu will be shown.
#[derive(Declare)]
pub struct MenuItem {
  #[declare(default)]
  pub disable: bool,
}

impl<'w> ComposeChild<'w> for MenuItem {
  type Child = MenuItemChild;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'w> {
    let MenuItemChild { label, leading, trailing, trailing_text: trailing_hint_text, sub_menu } =
      child;
    fn_widget! {
      let leading = leading.map(|w| {
        let w = FatObj::new(w.unwrap());
        @ $w { class: MENU_ITEM_LEADING }
      });
      let trailing_text = trailing_hint_text.map(
        |w| @Text{
          text: w.0,
          class: MENU_ITEM_HINT_TEXT
        }
      );
      let trailing = trailing.map(|w| {
        let w = FatObj::new(w.unwrap());
        @$w { class: MENU_ITEM_TRAILING }
      });
      let label = @Expanded{
        flex: 1.,
        @ Text{
          text: label,
          class: MENU_ITEM_LABEL
        }
      };

      let item = Provider::of::<MenuItemControl>(BuildCtx::get()).expect("MenuItem must be used within Menu").clone();
      let unsub = watch!($this.disable)
        .subscribe(move |v| {
          item.disable(v);
        });

      let class = Stateful::new(MENU_ITEM);
      @ Row{
        class: pipe!(*$class),
        align_items: Align::Center,
        on_disposed: {
          let sub_menu = sub_menu.clone();
          move |e| {
            unsub.unsubscribe();
            if let Some(menu) = sub_menu.as_ref() {
              if menu.is_show() {
                menu.close(&e.window());
              }
            }
          }
        },
        on_custom_concrete_event: move|e: &mut MenuEvent| {
          let wnd = e.window();
          match e.data() {
            MenuEventData::Select(selected, _) => {
              if *selected {
                *$class.write() = MENU_ITEM_SELECTED;
              } else {
                *$class.write() = MENU_ITEM;
                if let Some(menu) = sub_menu.as_ref() {
                  if menu.is_show() {
                    menu.close(&wnd);
                  }
                }
              }
            },
            MenuEventData::Enter(item) => {
              if let Some(menu) = sub_menu.as_ref() {
                if !menu.is_show() {
                  let id = e.current_target();
                  item.show_sub_menu(menu, id, &wnd);
                }
              }
            },
            _ => (),
          }
        },
        on_pointer_move: move |e| {
          let item = Provider::of::<MenuItemControl>(&e).expect("menuitem must in menu");
          item.enter(&e.window());
        },
        on_tap: move |e| {
          let item = Provider::of::<MenuItemControl>(&e).expect("menuitem must in menu");
          item.enter(&e.window());
        },
        @ { leading }
        @ { label }
        @ { trailing_text }
        @ { trailing }
      }
    }
    .into_widget()
  }
}

/// Menu item controller, the child of Menu can visit the MenuItemControl
/// through the provider
#[derive(Clone)]
pub struct MenuItemControl {
  idx: usize,
  menu: MenuControl,
}

impl MenuItemControl {
  /// emit the MenuEventData::Enter event
  pub fn enter(&self, wnd: &Sc<Window>) {
    if self.menu.select(Some(self.idx), wnd) {
      self.menu.enter(self.idx, wnd);
    }
  }

  /// check if the item is selected
  pub fn is_selected(&self) -> bool { self.menu.0.borrow().selected == Some(self.idx) }

  /// check if the item is disabled
  pub fn is_disabled(&self) -> bool { self.menu.0.borrow().items[self.idx].disabled }

  /// emit the MenuEventData::Complete event, normal it will bubble up and close
  /// all the menu. if you want to keep the menu, you should call
  /// stop_propagation of the event
  pub fn complete(&self, data: Option<impl Any>, wnd: &Sc<Window>) {
    let data = data.map(|v| Sc::new_any(v));
    self.bubble(MenuEventData::Complete(self.clone(), data.clone()), wnd);
  }

  /// close the current menu and not bubble up
  pub fn close(&self, wnd: &Sc<Window>) { self.menu.close(wnd); }

  /// disable the item, then it can't not be selected from the menu.
  pub fn disable(&self, disabled: bool) {
    self.menu.0.borrow_mut().items[self.idx].disabled = disabled;
  }

  /// show the sub menu around the widget
  pub fn show_sub_menu(&self, sub_menu: &MenuControl, around_wid: WidgetId, wnd: &Sc<Window>) {
    let pos = wnd.map_to_global(Point::zero(), around_wid);
    let size = wnd.widget_size(around_wid).unwrap();
    let rc = Rect::new(pos, size);
    sub_menu.show_around(rc, Some(self.clone()), wnd);
  }

  /// return the parent item if exists(for sub menu)
  pub fn parent(&self) -> Option<MenuItemControl> { self.menu.0.borrow().item_trigger.clone() }

  fn bubble(&self, data: MenuEventData, wnd: &Sc<Window>) {
    if let Some(from) = self.menu.0.borrow().items[self.idx].wid.get() {
      wnd.bubble_custom_event(from, data);
    }
  }
}

#[derive(Template)]
pub enum MenuChild {
  Item(Widget<'static>),
  Divider(MenuDivider),
}

/// MenuDivider
///
/// The MenuDivider can used to divide the menu items within the menu, which can
/// not be selected. If MenuDivider creates without a specified divider Widget,
/// it will use a default divider, otherwise, it will use the specified
/// widget as the divider.

#[derive(ChildOfCompose)]
#[simple_declare(stateless)]
pub struct MenuDivider {
  #[declare(default)]
  divider: Option<Widget<'static>>,
}

impl MenuDivider {
  fn into_divider_widget(self) -> Widget<'static> {
    self
      .divider
      .unwrap_or_else(|| fn_widget! { @Divider {} }.into_widget())
  }
}

fn wrap_menu_item(w: Widget<'static>, menu: &MenuControl) -> Widget<'static> {
  let menu = menu.clone();
  fn_widget! {
    let mut w = FatObj::new(w);
    let item = menu.new_item($w.track_id());
    @Providers {
      providers: smallvec![Provider::new(item)],
      @ { w }
    }
  }
  .into_widget()
}

impl ComposeChild<'static> for Menu {
  type Child = Vec<MenuChild>;
  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    fn_widget! {
      @Column {
        class: MENU,
        clip_boundary: true,
        on_disposed: move |e| {
          let menu = Provider::of::<MenuControl>(e).unwrap();
          menu.close(&e.window());
        },
        on_mounted: move |e| {
          e.window().request_focus(e.current_target());
        },
        on_key_down: move |e| {
          let menu = Provider::of::<MenuControl>(e).unwrap();
          match e.key() {
            VirtualKey::Named(NamedKey::ArrowUp) => {
              menu.select_next(false, &e.window());
            }
            VirtualKey::Named(NamedKey::ArrowDown) => {
              menu.select_next(true, &e.window());
            }
            VirtualKey::Named(NamedKey::Escape) => {
              menu.close(&e.window());
            }
            VirtualKey::Named(NamedKey::Enter) => {
              if let Some(idx) = menu.selected() {
                menu.enter(idx, &e.window());
              }
            }
            _ => {}
          }
        },
        @ {
          let menu = Provider::of::<MenuControl>(BuildCtx::get()).expect("Menu must in MenuControl");
          child.into_iter().map(move |w| match w {
            MenuChild::Item(w) => wrap_menu_item(w, &menu),
            MenuChild::Divider(w) => w.into_divider_widget(),
          })
        }
      }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::*, test_helper::*};

  use super::*;

  #[test]
  fn test_menu_item_selection() {
    reset_test_env!();
    let menu = MenuControl::new(menu! {
      @MenuItem  {@ { "Item 1" } }
      @MenuItem {
        disable: true,
        @ { "Item 2" }
      }
      @MenuItem { @ { "Item 3" } }
    });

    let widget = fn_widget! {
      @MockBox {
        size: Size::new(100., 100.),
      }
    };

    let mut wnd: TestWindow = TestWindow::new(widget);
    wnd.draw_frame();

    let raw_wnd = wnd.0.clone();
    menu.show(None, &raw_wnd);

    wnd.draw_frame();

    // Select the first item
    menu.select_next(true, &raw_wnd);
    assert_eq!(menu.selected(), Some(0));

    // Select the next item (should skip the disable)
    menu.select_next(true, &raw_wnd);
    assert_eq!(menu.selected(), Some(2));

    // Select the next item
    menu.select_next(true, &raw_wnd);
    assert_eq!(menu.selected(), Some(0));

    // Select the previous item
    menu.select_next(false, &raw_wnd);
    assert_eq!(menu.selected(), Some(2));
  }

  #[test]
  fn test_menu_item_enter() {
    reset_test_env!();
    let (r, w) = split_value(false);
    let menu = MenuControl::new(menu! {
      @MenuItem { @ { "Item 1" } }
      @MenuItem {
        on_custom_concrete_event: move|e: &mut MenuEvent| {
          if let MenuEventData::Enter(_) = e.data() {
              *$w.write() = true;
          }
        },
        @ { "Item 2" }
      }
    });

    let mut wnd: TestWindow = TestWindow::new(fn_widget! { @Void {} });
    wnd.draw_frame();

    let raw_wnd = wnd.0.clone();
    menu.show(None, &raw_wnd);
    wnd.draw_frame();

    // Enter the second item
    menu.enter(1, &raw_wnd);

    wnd.draw_frame();
    assert_eq!(menu.selected(), Some(1));
    assert!(*r.read());
  }

  #[test]
  fn test_sub_menu() {
    reset_test_env!();

    let (r, w) = split_value(String::new());
    let sub_menu = MenuControl::new(menu! {
      @MenuItem {
        on_custom_concrete_event: move|e: &mut MenuEvent| {
          if let MenuEventData::Enter(item) = e.data() {
            let s = "close from sub item".to_string();
            item.complete(Some(s), &e.window());
          }
        },
        @ { "Sub Item 1" }
      }
    });

    let sub_menu2 = sub_menu.clone();
    let menu = MenuControl::new(menu! {
        on_custom_concrete_event: move |e: &mut MenuEvent| {
          if let MenuEventData::Complete(_, Some(data)) = e.data() {
            if let Ok(s) = data.clone().downcast::<String>() {
              *$w.write() = s.to_string();
            }
          }
        },
        @MenuItem {
          @ { "Item 1" }
          @ { sub_menu.clone() }
        }
        @MenuItem { @ { "Item 2" } }
    });

    let mut wnd: TestWindow = TestWindow::new(fn_widget! { @Void {} });
    wnd.draw_frame();

    let raw_wnd = wnd.0.clone();
    menu.show(None, &raw_wnd);
    wnd.draw_frame();

    // Enter the first item to show the sub-menu

    menu.enter(0, &raw_wnd);
    wnd.draw_frame();

    assert!(sub_menu2.is_show());
    // Select the first sub-item
    sub_menu2.enter(0, &raw_wnd);
    wnd.draw_frame();

    assert!(!sub_menu2.is_show());
    assert!(!menu.is_show());
    assert_eq!(*r.read(), "close from sub item");
  }
}
