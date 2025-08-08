use ribir_core::prelude::{smallvec::SmallVec, *};
use ribir_widgets::{in_parent_layout, tabs::*};
use smallvec::smallvec;

use crate::*;

#[derive(Default, Clone)]
pub struct ActiveHeaderId(TrackId);

#[derive(Default, Clone)]
pub struct ActiveHeaderRect(Rect);

pub fn init(classes: &mut Classes) {
  class_names! {
    MD_INACTIVE_HEADER,
    MD_ACTIVE_HEADER,
  }

  classes.insert(
    TAB_HEADERS_VIEW,
    style_class! {
      h_align: tab_pos_var()
        .map(|pos| if pos.is_horizontal() { HAlign::Stretch } else { HAlign::Left }),
      v_align: tab_pos_var()
        .map(|pos| if !pos.is_horizontal() { VAlign::Stretch } else { VAlign::Top }),
    },
  );
  classes.insert(TAB_HEADERS_CONTAINER, |w| {
    stack! {
      providers: providers(),
      fit: StackFit::Passthrough,
      text_style: TypographyTheme::of(BuildCtx::get()).title_small.text.clone(),
      foreground: Palette::of(BuildCtx::get()).on_surface(),
      // divider
      border: {
        let color = Palette::of(BuildCtx::get()).surface_variant();
        tab_pos_var().map(move |pos| match pos {
          TabPos::Top => md::border_1_bottom(color),
          TabPos::Bottom => md::border_1_top(color),
          TabPos::Left => md::border_1_right(color),
          TabPos::Right => md::border_1_left(color),
        })
      },
      on_performed_layout: move |e|  {
        if let Some(active) = Provider::of::<ActiveHeaderId>(e).and_then(|i| i.0.get()) {
          let g_pos = e.map_from(Point::zero(), active);
          let size = e.widget_box_size(active).unwrap();
          Provider::write_of::<ActiveHeaderRect>(e).unwrap().0 = Rect::new(g_pos, size);
        }
      },
      @{ w }
      @in_parent_layout! {
        @ { tab_pos_var().map(indicator) }
      }
    }
    .into_widget()
  });

  // Configured the font style for the text within the `TAB_HEADERS_CONTAINER`
  // class.
  classes.insert(TAB_LABEL, empty_cls);
  classes.insert(
    TAB_ICON,
    style_class! {
      text_line_height: 24.,
      margin: inline_icon().map(|i| {
        if i.0 { md::EDGES_RIGHT_8 } else { EdgeInsets::default() }
      })
    },
  );

  classes.insert(TAB_HEADER, |w| {
    let w = class! {
      clamp: header_clamp(),
      class: is_active_header().map(move |active| {
        if active { MD_ACTIVE_HEADER } else { MD_INACTIVE_HEADER }
      }),
      cursor: CursorIcon::Pointer,
      foreground: foreground_color(),
      @{ w }
    };

    if DisabledRipple::get(BuildCtx::get()) {
      return w.into_widget();
    }

    interactive_layers! {
      bounded: true,
      @ { w }
    }
    .into_widget()
  });

  classes.insert(MD_INACTIVE_HEADER, style_class!(margin: md::EDGES_HOR_16));

  classes.insert(MD_ACTIVE_HEADER, |w| {
    let mut w = FatObj::new(w);

    // The active header's track id is written to the `ActiveHeaderId` provider.
    Provider::write_of::<ActiveHeaderId>(BuildCtx::get())
      .unwrap()
      .0 = w.track_id();

    let mut w = if tab_type() == TabType::Primary {
      // The primary tabs indicator's length depends on the active tab's content size.
      // Wrapping the tab header in a stack ensures the content retains its original
      // size.
      let stack = Stack::declarer().finish();
      w.with_margin(md::EDGES_HOR_16)
        .with_h_align(HAlign::Center)
        .with_v_align(VAlign::Center);
      stack.map(|p| p.with_child(w).into_widget())
    } else {
      w.with_padding(md::EDGES_HOR_16);
      w.map(IntoWidget::into_widget)
    };

    // This code is responsible for making sure the active tab header is visible
    // when the user navigates to it.
    w.on_mounted(|e| {
      let scrollable = ScrollableWidget::writer_of(e).unwrap();
      let wnd = e.window();
      let wid = e.current_target();

      wnd.clone().once_layout_ready(move || {
        if !wnd.is_valid_widget(wid) {
          return;
        }
        let mut scrollable = scrollable.write();
        let Some(pos) = scrollable.map_to_view(Point::zero(), wid, &wnd) else { return };
        let Some(size) = wnd.widget_size(wid) else { return };
        let header = Rect::new(pos, size);
        let view = scrollable.scroll_view_size();

        let min_space: f32 = 64.;
        let edge_gap = |size: f32, other_side_space| size.max(min_space).min(other_side_space);

        if scrollable.is_x_scrollable() {
          if header.max_x() + min_space > view.width && header.min_x() > 0. {
            let right = Anchor::right(edge_gap(header.width(), header.min_x()));
            scrollable.visible_widget(wid, right, &wnd);
          } else if header.min_x() < min_space && header.max_x() < view.width {
            let left = Anchor::left(edge_gap(header.width(), view.width - header.max_x()));
            scrollable.visible_widget(wid, left, &wnd);
          }
        }
        if scrollable.is_y_scrollable() {
          if header.max_y() + min_space > view.height && header.min_y() > 0. {
            let bottom = Anchor::bottom(edge_gap(header.height(), header.min_y()));
            scrollable.visible_widget(wid, bottom, &wnd);
          } else if header.min_y() < min_space && header.max_y() < view.height {
            let top = Anchor::top(edge_gap(header.height(), view.height - header.max_y()));
            scrollable.visible_widget(wid, top, &wnd);
          }
        }

        scrollable.visible_widget(wid, Anchor::default(), &wnd);
      });
    });
    w.into_widget()
  });
}

fn indicator(pos: &TabPos) -> Widget<'static> {
  fn p_length(length: f32) -> f32 { (length - 4.).max(24.) }
  fn p_offset(length: f32) -> f32 { (length - p_length(length)) / 2. }

  let header = active_header_rect_state();
  let tt = tab_type();
  let (size, anchor): (PipeValue<_>, PipeValue<_>) = match (tt, pos.is_horizontal()) {
    (TabType::Primary, true) => (
      distinct_pipe!(Size::new(p_length($read(header).width()), 3.)).r_into(),
      distinct_pipe!(Anchor::left($read(header).min_x() + p_offset($read(header).width())))
        .r_into(),
    ),
    (TabType::Primary, false) => (
      distinct_pipe!(Size::new(3., p_length($read(header).height()))).r_into(),
      distinct_pipe!(Anchor::top($read(header).min_y() + p_offset($read(header).height())))
        .r_into(),
    ),
    (_, true) => (
      distinct_pipe!(Size::new($read(header).width(), 2.)).r_into(),
      distinct_pipe!(Anchor::left($read(header).min_x())).r_into(),
    ),
    (_, false) => (
      distinct_pipe!(Size::new(2., $read(header).height())).r_into(),
      distinct_pipe!(Anchor::top($read(header).min_y())).r_into(),
    ),
  };

  rdl! {
    let mut indicator = @Container {
      size, background: BuildCtx::color(),
     };

     if tt == TabType::Primary {
      indicator.with_radius(match pos {
        TabPos::Top => Radius::top(3.),
        TabPos::Bottom => Radius::bottom(3.),
        TabPos::Left => Radius::left(3.),
        TabPos::Right => Radius::right(3.),
      });
    }

    let mut smooth = @SmoothPos {
      transition: EasingTransition {
        easing: md::easing::EMPHASIZED_DECELERATE,
        duration: md::easing::duration::MEDIUM1,
      },
      anchor
    };

    match pos {
      TabPos::Top =>  smooth.with_v_align(VAlign::Bottom),
      TabPos::Bottom =>  smooth.with_v_align(VAlign::Top),
      TabPos::Left =>  smooth.with_h_align(HAlign::Right),
      TabPos::Right =>  smooth.with_h_align(HAlign::Left),
    };
    @(smooth) {
      @NoAffectedParentSize { @IgnorePointer {
        @ @UnconstrainedBox {
          dir: UnconstrainedDir::Both,
          @ { indicator }
        }
      } }
    }
    .into_widget()
  }
}

fn tab_pos_var() -> Variant<TabPos> { Variant::<TabPos>::new_or_default(BuildCtx::get()) }

fn tabs_watcher() -> Box<dyn StateWatcher<Value = Tabs>> {
  Provider::state_of::<Box<dyn StateWatcher<Value = Tabs>>>(BuildCtx::get())
    .unwrap()
    .clone_watcher()
}

fn active_header_rect_state() -> Stateful<ActiveHeaderRect> {
  Provider::state_of::<Stateful<ActiveHeaderRect>>(BuildCtx::get())
    .unwrap()
    .clone_writer()
}

fn tab_type() -> TabType {
  Provider::of::<TabType>(BuildCtx::get())
    .map(|t| *t)
    .unwrap_or_default()
}

fn tab_info() -> TabInfo { *Provider::of::<TabInfo>(BuildCtx::get()).unwrap() }

fn header_clamp() -> BoxClamp {
  let height =
    if tab_type() == TabType::Primary && tab_info().has_icon_and_label() { 64. } else { 48. };
  BoxClamp::fixed_height(height)
}

fn inline_icon() -> Variant<TabsInlineIcon> {
  Variant::<TabsInlineIcon>::new_or_default(BuildCtx::get())
}

fn providers() -> SmallVec<[Provider; 1]> {
  let mut providers = smallvec![
    Provider::new(TextAlign::Center),
    Provider::writer(Stateful::new(ActiveHeaderId::default()), None),
    Provider::writer(Stateful::new(ActiveHeaderRect::default()), None),
  ];
  if tab_type() == TabType::Primary {
    providers.push(Provider::new(TabsInlineIcon(false)));
  }
  providers
}

fn foreground_color() -> PipeValue<Brush> {
  let ctx = BuildCtx::get();
  let tabs = tabs_watcher();
  let cur_tab = tab_info();
  let inactive_color = Palette::of(ctx).on_surface();
  let active_color = BuildCtx::color();

  active_color
    .map_with_watcher(tabs, move |active_color, tabs| {
      if tabs.active_idx() == cur_tab.idx { *active_color } else { inactive_color }
    })
    .r_into()
}

fn is_active_header() -> Pipe<bool> {
  let tabs = tabs_watcher();
  let cur_tab = tab_info();
  pipe!($read(tabs).active_idx() == cur_tab.idx)
}

impl std::ops::Deref for ActiveHeaderRect {
  type Target = Rect;
  fn deref(&self) -> &Self::Target { &self.0 }
}
