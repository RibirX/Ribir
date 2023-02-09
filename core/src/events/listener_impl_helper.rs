#[macro_export]
macro_rules! impl_listener {
  ($listener:ident, $declarer: ident, $field: ident, $event_ty: ident, $stream_name: ident) => {
    impl $declarer {
      pub fn $field(mut self, handler: impl for<'r> FnMut(&'r mut $event_ty) + 'static) -> Self {
        assert!(self.$field.is_none());
        let subject = MutRefItemSubject::default();
        subject.clone().subscribe(handler);
        self.$field = Some(subject);
        self
      }
    }

    impl Query for $listener {
      impl_query_self_only!();
    }

    impl $listener {
      /// Convert a observable stream of this event.
      pub fn $stream_name(&self) -> MutRefItemSubject<'static, $event_ty, ()> {
        self.$field.clone()
      }
    }

    impl EventListener for $listener {
      type Event = $event_ty;
      #[inline]
      fn dispatch(&self, event: &mut $event_ty) { self.$field.clone().next(event) }
    }
  };
}

#[macro_export]
macro_rules! impl_compose_child_for_listener {
  ($listener: ident) => {
    impl ComposeChild for $listener {
      type Child = Widget;
      #[inline]
      fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
        compose_child_as_data_widget(child, this)
      }
    }
  };
}

#[macro_export]
macro_rules! impl_compose_child_with_focus_for_listener {
  ($listener: ident) => {
    impl ComposeChild for $listener {
      type Child = Widget;
      fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
        let widget = dynamic_compose_focus_node(child);
        compose_child_as_data_widget(widget, this)
      }
    }
  };
}
