use ribir::prelude::*;

fn main() {
  let _circular_dependency_err = widget! {
    Flex {
      SizedBox {
        id: id1,
        size: id2.size,
      }
      SizedBox {
        id: id2,
        size: id3.size,
      }
      SizedBox {
        id: id3,
        size: id1.size,
      }
    }
  };

  let _wrap_widget_circular_err = widget! {
    SizedBox {
      id: parent,
      size: Size::zero(),
      margin: child.margin.clone(),
      SizedBox{
        id: child,
        size: Size::zero(),
        margin: parent.margin.clone(),
      }
    }
  };
}
