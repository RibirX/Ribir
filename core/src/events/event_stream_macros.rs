

macro_rules! impl_event_stream_dispatch {
  ($host: ident, $field: ident, $stream: ident, $event_ty: ty) => {
    impl EventListener for $host {
      type Event = $event_ty;
      fn dispatch(&self, event: Rc<RefCell<$event_ty>>) { self.$stream.clone().next(event) }
    }
  };
  ($host: ident, $field: ident, $event_ty: ty) => {
    paste::paste!(
      impl_event_stream_dispatch!($host, $field, [<$field _stream>], $event_ty);
    );
  }
}
  
macro_rules! impl_declare_listen_event {
  ($host: ident,  $field: ident, $stream: ident, $event_ty: ty) => {
    paste::paste!(impl [<$host Declarer>] {
      pub fn $field(self, f: impl FnMut(&mut $event_ty) + 'static) -> Self {
        let f_wrap: Rc<RefCell<Box<dyn  FnMut(&mut $event_ty)>>> = Rc::new(RefCell::new(Box::new(f)));
        let callback = f_wrap.clone();
        let stream = self.$stream.unwrap_or_default();
        stream
          .clone()
          .subscribe(move |event: Rc<RefCell<$event_ty>>| {
            (callback.borrow_mut())(&mut event.borrow_mut());
          });
        Self { $stream: Some(stream), $field: Some(f_wrap), ..self }
      }
    });
  };

  ($host: ident, $field: ident, $event_ty: ty) => {
    paste::paste!(
      impl_declare_listen_event!($host, $field, [<$field _stream>], $event_ty);
    );
  }
}

macro_rules! impl_set_declare_event_field {
  ($host: ident,  $field: ident, $event_ty: ty) => {
    paste::paste!(impl $host {
        pub fn [<set_declare_ $field>] (self, f: impl FnMut(&mut $event_ty) + 'static) -> Self {
        *self.$field.borrow_mut() = Box::new(f);
        self
      }
    });
  };
}

macro_rules! declare_builtin_event_field {
  ($host: ident, $field: ident, $stream: ident, $event_ty: ty) => {
      impl_set_declare_event_field!($host, $field, $event_ty);
      impl_declare_listen_event!($host, $field, $stream, $event_ty);
  };

  ($host: ident, $field: ident, $event_ty: ty) => {
    paste::paste!(
      declare_builtin_event_field!($host, $field, [<$field _stream>], $event_ty);
    );
  }
}