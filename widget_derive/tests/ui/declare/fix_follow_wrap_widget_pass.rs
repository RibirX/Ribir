use ribir::prelude::*;

fn main() {
  let _ = widget! {
    Row {
      id: _parent,
      background: son.background.clone(),
      margin: son.margin.clone(),
      SizedBox  {
        id: son,
        size: Size::new(100., 100.),
        margin: EdgeInsets::all(1.),
        background: Color::RED,
      }
    }
  };
}
