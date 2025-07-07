use std::cell::RefCell;

use ribir_core::prelude::*;
use smallvec::smallvec;

use crate::prelude::*;

class_names! {
  #[doc = "class name for the Menu"]
  MENU,
  #[doc="class name for Menu Item in unselected state"]
  MENU_ITEM,
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
  Select { selected: bool, idx: usize, label: CowArc<str>, menu: MenuControl },
  /// Emitted when the menu item is entered, the idx is the item
  /// that is triggered
  Enter { idx: usize, label: CowArc<str>, menu: MenuControl },
  /// Emitted when the menu is completed,
  /// the MenuItemControl is the item that is triggered,
  /// the Option<Sc<dyn Any>> is the data that is returned from the item.
  /// if the sub_menu's complete event is not stopped, the menu will be closed.
  Complete { idx: usize, label: CowArc<str>, menu: MenuControl, data: Option<Sc<dyn Any>> },
}

/// the menu event will be emitted from the menu item that is triggered
pub type MenuEvent = CustomEvent<MenuEventData>;

/// Menu, must be use within the MenuControl.
///
/// You can use the built-in [`MenuItem`] to create a menu. And use
/// [`MenuDivider`] to create a divider in the menu, and listen to the menu
/// event
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
///     on_custom_concrete_event: move |e: &mut MenuEvent| {
///       if matches!( e.data(), MenuEventData::Enter {..}) {
///         println!("Enter");
///       }
///     },
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
///       menu.show_at(e.position(), &e.window());
///     },
///   }
/// };
/// App::run(w);
/// ```
#[derive(Declare)]
pub struct Menu {}

/// the controller of the popup menu
#[derive(Clone)]
pub struct MenuControl(Sc<RefCell<MenuData>>);

struct MenuItemData {
  wid: TrackId,
  label: CowArc<str>,
}

struct MenuData {
  id: Option<TrackId>,
  handle: Option<Overlay>,
  item_trigger: Option<ParentMenuInfo>,
  selected: Option<usize>,
  items: Vec<MenuItemData>,
  gen: GenWidget,
}

impl MenuControl {
  /// Receive a function generator of widget return a MenuControl
  pub fn new<K: ?Sized>(gen: impl RInto<GenWidget, K>) -> Self {
    Self(Sc::new(RefCell::new(MenuData {
      gen: gen.r_into(),
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
  pub fn show(&self, wnd: &Sc<Window>) {
    let gen = self.0.borrow().gen.clone();
    self.inner_show(gen, None, wnd);
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
      wnd.request_focus(id, FocusReason::Other);
    }
  }

  /// Show the menu around the target rect, the target rect is relative to the
  /// window
  pub fn show_around(&self, target: Rect, wnd: &Sc<Window>) {
    self.show_map(anchor_around(target), wnd);
  }

  /// Show the menu around the global position
  pub fn show_at(&self, pos: Point, wnd: &Sc<Window>) {
    self.show_map(anchor_around(Rect::new(pos, Size::zero())), wnd);
  }

  pub fn show_map<F>(&self, mut f: F, wnd: &Sc<Window>)
  where
    F: FnMut(Widget<'static>) -> Widget<'static> + 'static,
  {
    let gen = self.0.borrow().gen.clone();
    let gen = GenWidget::new(move || f(gen.gen_widget()));
    self.inner_show(gen, None, wnd);
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
      let idx = if let Some(idx) = old_index { idx } else { start_idx };
      Some((idx + offset) % len)
    };
    let idx = calc_next_idx(&self.0.borrow());
    self.select(idx, wnd);
  }

  /// Select the nth Selectable item.
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
  pub fn select(&self, idx: Option<usize>, wnd: &Sc<Window>) -> bool {
    let mut this = self.0.borrow_mut();
    if this.selected == idx {
      return true;
    }
    if let Some(selected) = this.selected {
      let label = this.items[selected].label.clone();
      if let Some(from) = this.items[selected].wid.get() {
        wnd.bubble_custom_event(
          from,
          MenuEventData::Select { selected: false, idx: selected, label, menu: self.clone() },
        );
      }
    }

    if let Some(selected) = idx {
      let label = this.items[selected].label.clone();
      if let Some(from) = this.items[selected].wid.get() {
        wnd.bubble_custom_event(
          from,
          MenuEventData::Select { selected: true, idx: selected, label, menu: self.clone() },
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
    let label = self.0.borrow().items[idx].label.clone();
    if let Some(from) = wid {
      wnd.bubble_custom_event(from, MenuEventData::Enter { idx, label, menu: self.clone() });
    }
    true
  }

  fn selected(&self) -> Option<usize> { self.0.borrow().selected }

  fn inner_show(&self, gen: GenWidget, parent: Option<ParentMenuInfo>, wnd: &Sc<Window>) {
    let handle = self.clone();
    let fn_gen = GenWidget::from_fn_widget(fn_widget! {
      let mut w = FatObj::new(gen.clone());
      handle.0.borrow_mut().id = Some(w.track_id());
      @Providers {
        providers: smallvec![Provider::new(handle.clone())],
        @(w) {
          on_custom_concrete_event: move |e: &mut MenuEvent| {
            if let MenuEventData::Complete{menu,  data, ..} = e.data() {
              if let Some(ParentMenuInfo {menu, idx}) = menu.0.borrow().item_trigger.as_ref() {
                let item = &menu.0.borrow().items[*idx];
                if let Some(wid) = item.wid.get() {
                  let label = item.label.clone();
                  let data = data.clone();
                  e.window().bubble_custom_event(
                    wid,
                    MenuEventData::Complete{idx: *idx, label, menu: menu.clone(), data}
                  );
                }
              }
              menu.close(&e.window());
            }
          },
        }
      }
    });

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

  fn new_item(&self, wid: TrackId, key: CowArc<str>) -> usize {
    self
      .0
      .borrow_mut()
      .items
      .push(MenuItemData { wid, label: key });
    let idx = self.0.borrow().items.len() - 1;
    idx
  }

  fn show_sub_menu(
    &self, from_item: usize, sub_menu: &MenuControl, around_wid: WidgetId, wnd: &Sc<Window>,
  ) {
    let pos = wnd.map_to_global(Point::zero(), around_wid);
    let size = wnd.widget_size(around_wid).unwrap();
    let rc = Rect::new(pos, size);
    let gen = sub_menu.0.borrow().gen.clone();

    sub_menu.inner_show(
      GenWidget::new(move || anchor_around(rc)(gen.gen_widget())),
      Some(ParentMenuInfo { menu: self.clone(), idx: from_item }),
      wnd,
    );
  }

  // emit MenuEventData::Complete
  pub fn complete(&self, label: CowArc<str>, data: Option<Sc<dyn Any>>, wnd: &Sc<Window>) {
    let this = self.0.borrow();
    if let Some(idx) = this
      .items
      .iter()
      .position(|item| item.label == label)
    {
      if let Some(wid) = this.items[idx].wid.get() {
        wnd.bubble_custom_event(
          wid,
          MenuEventData::Complete { idx, label, menu: self.clone(), data },
        );
      }
    }
  }
}

fn anchor_around(target: Rect) -> impl FnMut(Widget<'static>) -> Widget<'static> {
  move |w: Widget<'static>| -> Widget<'static> {
    fn_widget! {
      let mut w = FatObj::new(w);
      @(w) {
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

pub struct MenuHintText(TextValue);
impl MenuHintText {
  pub fn new<K: ?Sized>(child: impl RInto<TextValue, K>) -> Self { MenuHintText(child.r_into()) }
}

#[derive(Template)]
pub struct MenuItem<'w> {
  /// the label string of this menu item, if the custom widget is not specified,
  /// it will be used as the label widget
  label: CowArc<str>,
  /// custom widget, if not specified, the label will be showed.
  custom: Option<Widget<'w>>,
  /// trailing hint text
  trailing_text: Option<MenuHintText>,
  /// leading icon
  leading: Option<Leading<Widget<'w>>>,
  /// trailing icon
  trailing: Option<Trailing<Widget<'static>>>,
  /// sub menu
  sub_menu: Option<MenuControl>,
}

impl<'w> MenuItem<'w> {
  fn into_widget(self) -> Widget<'w> {
    let MenuItem { label, custom, leading, trailing, trailing_text: trailing_hint_text, sub_menu } =
      self;
    fn_widget! {
      let leading = leading.map(|w| {
        let mut w = FatObj::new(w.unwrap());
        @(w) { class: MENU_ITEM_LEADING }
      });
      let trailing_text = trailing_hint_text.map(
        |w| @Text{
          text: w.0,
          class: MENU_ITEM_HINT_TEXT
        }
      );
      let trailing = trailing.map(|w| {
        let mut w = FatObj::new(w.unwrap());
        @(w) { class: MENU_ITEM_TRAILING }
      });

      let content = custom.unwrap_or_else(|| {
        @Expanded{
          flex: 1.,
          @ Text{
            text: label.clone(),
            class: MENU_ITEM_LABEL
          }
        }.into_widget()
      });

      let class = Stateful::new(MENU_ITEM);
      @Row{
        class: pipe!(*$read(class)),
        align_items: Align::Center,
        on_disposed: {
          let sub_menu = sub_menu.clone();
          move |e| {
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
            MenuEventData::Select{ selected, .. } => {
              if *selected {
                *$write(class) = MENU_ITEM_SELECTED;
              } else {
                *$write(class) = MENU_ITEM;
                if let Some(menu) = sub_menu.as_ref() {
                  if menu.is_show() {
                    menu.close(&wnd);
                  }
                }
              }
            },
            MenuEventData::Enter{ idx, menu, .. } => {
              if let Some(sub_menu) = sub_menu.as_ref() {
                if !sub_menu.is_show() {
                  let id = e.current_target();
                  menu.show_sub_menu(*idx, sub_menu, id, &wnd);
                }
              }
            },
            _ => (),
          }
        },

        @ { leading }
        @ { content }
        @ { trailing_text }
        @ { trailing }
      }
    }
    .into_widget()
  }
}

#[derive(Clone)]
struct ParentMenuInfo {
  idx: usize,
  menu: MenuControl,
}

#[derive(Template)]
pub enum MenuChild<'w> {
  Item(MenuItem<'w>),
  Divider(MenuDivider),
}

/// MenuDivider
///
/// The MenuDivider can used to divide the menu items within the menu, which can
/// not be selected. If MenuDivider creates without a specified divider Widget,
/// it will use a default divider, otherwise, it will use the specified
/// widget as the divider.

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

fn wrap_menu_item<'w>(w: Widget<'w>, key: CowArc<str>, menu: &MenuControl) -> Widget<'w> {
  let menu = menu.clone();
  fn_widget! {
    let mut w = FatObj::new(w);
    let idx = menu.new_item(w.track_id(), key);
    @(w) {
      on_pointer_move: {
        let menu = menu.clone();
        move |e| {
          menu.enter(idx, &e.window());
        }
      },
      on_tap: {
        let menu = menu.clone();
        move |e| {
          menu.enter(idx, &e.window());
        }
      },
    }
  }
  .into_widget()
}

impl<'w> ComposeChild<'w> for Menu {
  type Child = Vec<MenuChild<'w>>;
  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'w> {
    fn_widget! {
      @Column {
        class: MENU,
        clip_boundary: true,
        on_disposed: move |e| {
          let menu = Provider::of::<MenuControl>(e).unwrap();
          menu.close(&e.window());
        },
        on_mounted: move |e| {
          e.window().request_focus(e.current_target(), FocusReason::AutoFocus);
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
            MenuChild::Item(w) => {
              let key = w.label.clone();
              wrap_menu_item(w.into_widget(), key, &menu)
            },
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
      @MenuItem { @ { "Item 1" } }
      @MenuItem { @ { "Item 2" } }
    });

    let widget = fn_widget! {
      @MockBox {
        size: Size::new(100., 100.),
      }
    };

    let wnd: TestWindow = TestWindow::from_widget(widget);
    wnd.draw_frame();

    let raw_wnd = wnd.0.clone();
    menu.show(&raw_wnd);

    wnd.draw_frame();

    // Select the first item
    menu.select_next(true, &raw_wnd);
    assert_eq!(menu.selected(), Some(0));

    // Select the next item
    menu.select_next(true, &raw_wnd);
    assert_eq!(menu.selected(), Some(1));

    // Select the next item
    menu.select_next(true, &raw_wnd);
    assert_eq!(menu.selected(), Some(0));

    // Select the previous item
    menu.select_next(false, &raw_wnd);
    assert_eq!(menu.selected(), Some(1));
  }

  #[test]
  fn test_menu_item_enter() {
    reset_test_env!();
    let (r, w) = split_value(false);
    let menu = MenuControl::new(menu! {
      on_custom_concrete_event: move|e: &mut MenuEvent| {
        if let MenuEventData::Enter{idx, ..} = e.data() {
          if *idx == 1 { *$write(w) = true; }
        }
      },
      @MenuItem { @ { "Item 1" } }
      @MenuItem { @ { "Item 2" } }
    });

    let wnd: TestWindow = TestWindow::from_widget(fn_widget! { @Void {} });
    wnd.draw_frame();

    let raw_wnd = wnd.0.clone();
    menu.show(&raw_wnd);
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
      on_custom_concrete_event: move|e: &mut MenuEvent| {
        if let MenuEventData::Enter{ menu, label,.. } = e.data() {
          let s = "close from sub item".to_string();
          menu.complete(label.clone(), Some(Sc::new_any(s)), &e.window());
        }
      },
      @ MenuItem {
        @ { "Sub Item 1" }
        @ { Void {} }
      }
    });

    let sub_menu2 = sub_menu.clone();
    let menu = MenuControl::new(menu! {
        on_custom_concrete_event: move |e: &mut MenuEvent| {
          if let MenuEventData::Complete{data: Some(data), ..} = e.data() {
            if let Ok(s) = data.clone().downcast::<String>() {
              *$write(w) = s.to_string();
            }
          }
        },
        @MenuItem {
          @ { "Item 1" }
          @ { sub_menu.clone() }
        }

        @MenuItem { @ { "Item 2" } }
    });

    let wnd: TestWindow = TestWindow::from_widget(fn_widget! { @Void {} });
    wnd.draw_frame();

    let raw_wnd = wnd.0.clone();
    menu.show(&raw_wnd);
    wnd.draw_frame();

    // Enter the first item to show the sub-menu
    assert!(!sub_menu2.is_show());

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
