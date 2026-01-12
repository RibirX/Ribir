use crate::prelude::*;

/// A widget that allows its child to follow the position of another widget.
/// This replaces the removed `Global` positioning in `PosX`/`PosY`.
#[derive(Declare, Clone)]
pub struct Follow {
  pub target: TrackId,
  #[declare(default)]
  pub x_align: AnchorX,
  #[declare(default)]
  pub y_align: AnchorY,
}

impl<'c> ComposeChild<'c> for Follow {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut child = FatObj::new(child);
      let wnd = BuildCtx::get().window();
      let x_custom = {
        let wnd = wnd.clone();
        let target = $read(this).target.clone();
        PosX::custom(move |_, self_width| {
           if let Some(tid) = target.get() {
              let target_size = wnd.widget_size(tid).unwrap_or_default();
              let target_pos = wnd.widget_pos(tid).unwrap_or_default();
              let global_pos = tid.parent(wnd.tree())
                .map(|p| wnd.map_to_global(target_pos, p))
                .unwrap_or(target_pos);

              global_pos.x + $read(this).x_align.calculate(target_size.width, self_width)
          } else {
            0.
          }
        })
      };

      let y_custom = {
        let wnd = wnd.clone();
        let target = $read(this).target.clone();
        PosY::custom(move |_, self_height| {
          if let Some(tid) = target.get() {
            let target_size = wnd.widget_size(tid).unwrap_or_default();
            let target_pos = wnd.widget_pos(tid).unwrap_or_default();
            let global_pos = tid.parent(wnd.tree())
              .map(|p| wnd.map_to_global(target_pos, p))
              .unwrap_or(target_pos);

            global_pos.y + $read(this).y_align.calculate(target_size.height, self_height)
          } else {
            0.
          }
        })
      };

      child.with_x(x_custom);
      child.with_y(y_custom);
      child.into_widget()
    }
    .into_widget()
  }
}
