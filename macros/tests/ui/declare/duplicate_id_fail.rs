use ribir::prelude::*;

fn main() {
  let _id_must_be_unique_err = widget! {
    BoxDecoration {
      id: same_id,
      background: Some(Color::RED.into()),
      SizedBox {
        id: same_id,
        size: Size::zero(),
      }
    }
  };

  let _id_conflict_with_states_err = widget! {
    states { same_id: Stateful::new(0) }
    BoxDecoration {
      id: same_id,
      background: Some(Color::RED.into()),
    }
  };

  let _inner_id_conflict_outside_err = widget! {
    BoxDecoration {
      id: same_id,
      background: Some(Color::RED.into()),
      DynWidget {
        dyns: widget!{
          SizedBox { id: same_id, size: Size::zero() }
        }
      }
    }
  };

  let _inner_id_conflict_outside_states_err = widget! {
    states { same_id: Stateful::new(0),}
    DynWidget {
      dyns: widget!{
        SizedBox { id: same_id, size: Size::zero(),}
      }
    }
  };

  let _inner_states_id_conflict_outside_states_err = widget! {
    states { same_id: Stateful::new(0),}
    DynWidget {
      dyns: widget!{
        states { same_id: Stateful::new(0),}
        SizedBox { size: Size::zero(),}
      }
    }
  };

  let _inner_states_id_conflict_outside_id_err = widget! {
    BoxDecoration {
      id: same_id,
      background: Some(Color::RED.into()),
      DynWidget {
        dyns: widget!{
          states { same_id: Stateful::new(0),}
          SizedBox { size: Size::zero(),}
        }
      }
    }
  };
}
