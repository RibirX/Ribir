use ribir::prelude::*;

use crate::{
  pomodoro::{Pomodoro, UPDATE_INTERVAL},
  smallvec::SmallVec,
  ui::{PomodoroPage, UiState, styles::*, widgets::*},
};

/// Settings button icon that toggles between settings and back arrow
fn setting_button_icon() -> Widget<'static> {
  fn_widget! {
    let ui_state = Provider::writer_of::<UiState>(BuildCtx::get()).unwrap();
    @Icon {
      cursor: CursorIcon::Pointer,
      visible: pipe!($read(ui_state).current_page != PomodoroPage::Mini),
      on_tap: move |_| {
        let current = $read(ui_state).current_page;
        $write(ui_state).current_page = match current {
          PomodoroPage::Mini | PomodoroPage::Main =>  PomodoroPage::Setting,
          PomodoroPage::Setting => PomodoroPage::Main,
        };
      }
      @ {
        pipe!($read(ui_state).current_page).map(move |v| match v {
          PomodoroPage::Mini | PomodoroPage::Main => svgs::SETTINGS,
          PomodoroPage::Setting => svgs::ARROW_BACK,
        })
      }
    }
  }
  .into_widget()
}

impl Compose for Pomodoro {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      this.write().reset();

      let config = this.read().config.clone();
      let ui_state = Stateful::new(UiState {
        current_page: if config.start_mini_mode {
          PomodoroPage::Mini
        } else {
          PomodoroPage::Main
        },
        keep_on_top: config.always_on_top,
      });

      let wnd = BuildCtx::get().window();
      if config.auto_run {
        Pomodoro::run(&this, UPDATE_INTERVAL);
      }
      if ui_state.read().keep_on_top {
        wnd.set_window_level(WindowLevel::AlwaysOnTop);
      }

      watch!($read(ui_state).current_page == PomodoroPage::Mini)
        .distinct_until_changed()
        .subscribe(move |v| {
          if v {
            wnd.request_resize(Size::new(MINI_WIDTH, MINI_HEIGHT));
          } else {
            wnd.request_resize(Size::new(FULL_WIDTH, FULL_HEIGHT));
          }
        });

      Overlay::new(fn_widget! {
        @(FatObj::new(())) {
          providers: [Provider::writer($writer(ui_state), Some(DirtyPhase::LayoutSubtree))],
          global_anchor_x: GlobalAnchorX::value(HAnchor::Left(6.0.into())),
          global_anchor_y: GlobalAnchorY::value(VAnchor::Top(30.0.into())),
          @ { setting_button_icon() }
        }
      }, OverlayStyle {
          mask: None,
          auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE,
      }).show(BuildCtx::get().window());

      @Column {
        providers: {
          let mut ps = vec!(
            Provider::writer(this.clone_writer(), None),
            Provider::writer(ui_state.clone_writer(), Some(DirtyPhase::LayoutSubtree)),
          );
          ps.extend(styles());
          SmallVec::from_vec(ps)
        },
        @WindowBar {}
        @ {
          pipe!($read(ui_state).current_page)
            .transform(|p| p.distinct_until_changed().box_it())
            .map(move |b| match b {
              PomodoroPage::Main => main_page(),
              PomodoroPage::Mini => concise_page(),
              PomodoroPage::Setting => setting_page(),
            })
        }
      }
    }
    .into_widget()
  }
}
