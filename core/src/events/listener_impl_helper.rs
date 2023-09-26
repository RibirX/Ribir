#[macro_export]
macro_rules! impl_all_event {
  ($name: ident, $($on_doc: literal, $event_ty: ident),+) => {
    paste::paste! {
      #[doc="All `" $name:snake "` related events"]
      pub enum [<All $name>] {
        $(
          #[doc = $on_doc]
          $event_ty([<$name Event>]),
        )+
      }

      impl std::ops::Deref for [<All $name>] {
        type Target = [<$name Event>];
        fn deref(&self) -> &Self::Target {
          match self {
            $([<All $name>]::$event_ty(e)) |+ => e
          }
        }
      }

      impl std::ops::DerefMut for [<All $name>] {
        fn deref_mut(&mut self) -> &mut Self::Target {
          match self {
            $([<All $name>]::$event_ty(e)) |+ => e
          }
        }
      }

      impl [<All $name>] {
        pub fn into_inner(self) -> [<$name Event>] {
          match self {
            $([<All $name>]::$event_ty(e)) |+ => e
          }
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_listener {
  ($doc: literal, $name: ident, $event_ty: ident) => {
    paste::paste! {
      #[doc= $doc]
      #[derive(Query)]
      pub struct [<$name Listener>]{
        [<$name:snake _subject>]: [<$name Subject>]
      }

      impl [<$name Listener>] {
        fn subject(&mut self) -> [<$name Subject>] {
          self
            .[<$name:snake _subject>]
            .clone()
        }
      }

      impl Declare2 for [<$name Listener>] {
        type Builder = Self;
        fn declare2_builder() -> Self::Builder {
          Self { [<$name:snake _subject>]: Default::default()}
        }
      }

      impl DeclareBuilder for [<$name Listener>] {
        type Target = State<Self>;
        fn build_declare(self, _ctx: &BuildCtx) -> Self::Target { State::value(self) }
      }

      impl [<$name Listener>] {
        /// Convert a observable stream of this event.
        pub fn [<$name:snake _stream>](&self) -> [<$name Subject>] {
          self.[<$name:snake _subject>].clone()
        }
      }

      impl EventListener for [<$name Listener>] {
        type Event = $event_ty;
        #[inline]
        fn dispatch(&self, event: &mut Self::Event) {
          self.[<$name:snake _subject>].clone().next(event)
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_multi_event_listener {
  (
    $doc: literal, $name: ident,
    $($on_doc: literal, $event_ty: ident),+
  ) => {
    paste::paste! {
      impl_all_event!($name, $($on_doc, $event_ty),+);
      impl_listener!($doc, $name, [<All $name>]);

      impl [<$name Listener>] {
        $(
          #[doc = "Sets up a function that will be called whenever the `" $event_ty "` is delivered"]
          pub fn [<on_ $event_ty:snake>](
            mut self,
            handler: impl FnMut(&mut [<$name Event>]) + 'static
          ) -> Self
          {
            self
            .subject()
            .filter_map(
              (|e| match e {
                [<All $name>]::$event_ty(e) => Some(e),
                _ => None,
              }) as fn(&mut [<All $name>]) -> Option<&mut [<$name Event>]>
            )
            .subscribe(handler);
            self
          }
        )+
      }
    }
  };
}

#[macro_export]
macro_rules! impl_single_event_listener {
  ($doc: literal, $name: ident) => {
    paste::paste! {
      impl_listener!($doc, $name);

      impl [<$name ListenerDeclarer2>] {
        #[doc = "Sets up a function that will be called whenever the `" [<$name Event>] "` is delivered"]
        pub fn [<on_ $name:snake>](
          self,
          handler: impl FnMut(&'_ mut [<$name Event>]<'_>) + 'static
        ) -> Self {
          self
            .subject()
            .subscribe(handler);
          self
        }
      }

      impl [<$name ListenerDeclarer>] {
        #[doc = "Sets up a function that will be called whenever the `" [<$name Event>] "` is delivered"]
        pub fn [<on_ $name:snake>](
          self,
          handler: impl FnMut(&'_ mut [<$name Event>]<'_>) + 'static
        ) -> Self {
          self
            .subject()
            .subscribe(handler);
          self
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_common_event_deref {
  ($event_name: ident) => {
    impl std::ops::Deref for $event_name {
      type Target = CommonEvent;

      #[inline]
      fn deref(&self) -> &Self::Target { &self.common }
    }

    impl std::ops::DerefMut for $event_name {
      #[inline]
      fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
    }

    impl std::borrow::Borrow<CommonEvent> for $event_name {
      #[inline]
      fn borrow(&self) -> &CommonEvent { &self.common }
    }

    impl std::borrow::BorrowMut<CommonEvent> for $event_name {
      #[inline]
      fn borrow_mut(&mut self) -> &mut CommonEvent { &mut self.common }
    }
  };
}

#[macro_export]
macro_rules! impl_compose_child_for_listener {
  ($listener: ident) => {
    impl ComposeChild for $listener {
      type Child = Widget;
      #[inline]
      fn compose_child(
        this: impl StateWriter<Value = Self>,
        child: Self::Child,
      ) -> impl WidgetBuilder {
        move |ctx: &BuildCtx| child.attach_state_data(this, ctx)
      }
    }
  };
}

#[macro_export]
macro_rules! impl_compose_child_with_focus_for_listener {
  ($listener: ident) => {
    impl ComposeChild for $listener {
      type Child = Widget;
      fn compose_child(
        this: impl StateWriter<Value = Self>,
        child: Self::Child,
      ) -> impl WidgetBuilder {
        fn_widget! {
          @FocusNode {
            tab_index: 0i16, auto_focus: false,
            @ { child.attach_state_data(this, ctx!()) }
          }
        }
      }
    }
  };
}
