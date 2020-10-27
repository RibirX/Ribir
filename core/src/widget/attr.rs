use crate::prelude::*;
use std::{any::Any, fmt::Debug, marker::PhantomData};

/// WidgetAttr is use to extend ability of a widget but not increase the widget
/// number. If a widget is not a combination widget and will not do layout or
/// paint, it should be consider as a WidgetAttr. Like the event listeners,
/// `KeyDetect`, `Stateful` and so on.
///
/// WidgetAttr attach the ability to a widget, if many `WidgetAttr` attached to
/// a same widget, they are organized like a linked list, an `WidgetAttr` hold
/// another `WidgetAttr` until the `WidgetAttr` hold a real widget.
///
/// ## Notice
/// When you implement a new `WidgetAttr`, you should remember a widget can only
/// attach one attr of same `WidgetAttr` type. If user attach it many times, you
/// should merge them.
#[derive(Debug)]
pub struct WidgetAttr<W: Widget, AttrData> {
  pub attr: AttrData,
  widget: BoxWidget,
  type_info: PhantomData<*const W>,
}

pub trait Attribute: Widget {
  fn widget(&self) -> &BoxWidget;
  fn widget_mut(&mut self) -> &mut BoxWidget;
}

pub enum AttrOrWidget<W: Widget, A> {
  Attr(WidgetAttr<W, A>),
  Widget(BoxWidget),
}

pub trait AttributeAttach: Widget {
  type HostWidget: Widget;

  /// Assign a key to the widget to help framework to track if two widget is a
  /// same widget in two frame.
  #[inline]
  fn with_key<K: Into<Key> + 'static>(self, key: K) -> KeyDetect<Self::HostWidget>
  where
    Self: Sized,
  {
    let key = key.into();
    match self.pop_attr() {
      AttrOrWidget::Attr(mut attr) => {
        attr.attr = key;
        attr
      }
      AttrOrWidget::Widget(widget) => KeyDetect {
        widget,
        attr: key,
        type_info: PhantomData,
      },
    }
  }

  /// Convert a stateless widget to stateful, and will split to a stateful
  /// widget, and a `StateRef` which can be use to modify the states of the
  /// widget.
  #[inline]
  fn into_stateful(self) -> Stateful<Self::HostWidget>
  where
    Self: Sized,
  {
    match self.pop_attr() {
      AttrOrWidget::Attr(attr) => attr,
      AttrOrWidget::Widget(mut widget) => {
        let attr = widget::stateful::StatefulAttr::new(&mut widget);

        Stateful {
          widget,
          attr,
          type_info: PhantomData,
        }
      }
    }
  }

  /// If this widget is has the `AttrData` attribute, this method pop the
  /// `AttrData` to the most outside, and return it, otherwise return a
  /// `BoxWidget`
  fn pop_attr<AttrData: 'static>(self) -> AttrOrWidget<Self::HostWidget, AttrData>
  where
    Self: Sized,
  {
    let mut boxed = self.box_it();
    // Safety: if we success copy the attribute, we will forget the origin object.
    if let Some((widget, attr)) = unsafe { copy_attr(&mut boxed) } {
      std::mem::forget(boxed);
      AttrOrWidget::Attr(WidgetAttr {
        attr,
        widget,
        type_info: PhantomData,
      })
    } else {
      let mut target = boxed.as_attr_mut();
      let mut attr = None;
      while let Some(attr_widget) = target.take() {
        // Safety: if we success copy the attribute, we will forget the origin object.
        if let Some((widget, a)) = unsafe { copy_attr(attr_widget.widget_mut()) } {
          let detached = std::mem::replace(attr_widget.widget_mut(), widget);
          std::mem::forget(detached);
          attr = Some(a);
          break;
        } else {
          target = attr_widget.widget_mut().as_attr_mut();
        }
      }

      if let Some(attr) = attr {
        AttrOrWidget::Attr(WidgetAttr {
          attr,
          widget: boxed,
          type_info: PhantomData,
        })
      } else {
        AttrOrWidget::Widget(boxed)
      }
    }
  }
}

impl<W: Widget, AttrData: Any + Debug> Attribute for WidgetAttr<W, AttrData> {
  #[inline]
  fn widget(&self) -> &BoxWidget { &self.widget }

  #[inline]
  fn widget_mut(&mut self) -> &mut BoxWidget { &mut self.widget }
}

impl<W: Widget, Data: Any + Debug> Widget for WidgetAttr<W, Data> {
  fn classify(&self) -> WidgetClassify { self.widget.classify() }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { self.widget.classify_mut() }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  #[inline]
  fn as_attr(&self) -> Option<&dyn Attribute>
  where
    Self: Sized,
  {
    Some(self)
  }

  #[inline]
  fn as_attr_mut(&mut self) -> Option<&mut dyn Attribute>
  where
    Self: Sized,
  {
    Some(self)
  }

  fn box_it(self) -> BoxWidget
  where
    Self: Sized,
  {
    let erase_type: WidgetAttr<BoxWidget, Data> = WidgetAttr {
      widget: self.widget,
      attr: self.attr,
      type_info: PhantomData,
    };
    let widget: Box<dyn Widget> = Box::new(erase_type);
    widget.into()
  }
}

unsafe fn copy_attr<AttrData: 'static>(widget: &mut BoxWidget) -> Option<(BoxWidget, AttrData)> {
  if let Some(attr) = widget
    .as_any_mut()
    .downcast_mut::<WidgetAttr<BoxWidget, AttrData>>()
  {
    let mut tmp = std::mem::MaybeUninit::uninit();
    let ptr: *mut WidgetAttr<BoxWidget, AttrData> = tmp.as_mut_ptr();
    ptr.copy_from(
      attr as *const WidgetAttr<BoxWidget, AttrData>,
      std::mem::size_of::<WidgetAttr<BoxWidget, AttrData>>(),
    );
    let tmp = tmp.assume_init();
    Some((tmp.widget, tmp.attr))
  } else {
    None
  }
}

impl<W: Widget, Data: Debug + 'static> AttributeAttach for WidgetAttr<W, Data> {
  type HostWidget = W;
}

impl<W: Widget, Attr: Any + Debug> std::ops::Deref for WidgetAttr<W, Attr> {
  type Target = W;
  fn deref(&self) -> &Self::Target {
    let mut widget: &dyn Widget = self;
    while let Some(attr) = widget.as_attr() {
      widget = attr.widget();
    }
    debug_assert_eq!(
      widget.as_any().type_id(),
      std::any::TypeId::of::<Self::Target>()
    );
    let ptr = widget as *const dyn Widget as *const Self::Target;

    // Safety: the type info always hold the origin widget type.
    unsafe { &*ptr }
  }
}

impl<W: Widget, Attr: Any + Debug> std::ops::DerefMut for WidgetAttr<W, Attr> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    let mut widget = self as *mut dyn Widget;
    // Safety: the type info always hold the origin widget type.
    unsafe {
      while let Some(attr) = (&mut *widget).as_attr_mut() {
        widget = attr.widget_mut() as *mut dyn Widget;
      }

      debug_assert_eq!(
        (*widget).as_any().type_id(),
        std::any::TypeId::of::<Self::Target>()
      );

      let ptr = widget as *mut Self::Target;
      &mut *ptr
    }
  }
}
