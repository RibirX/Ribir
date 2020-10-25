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
  pub widget: BoxWidget,
  pub attr: AttrData,
  pub marker: PhantomData<*const W>,
}

pub trait Attribute: Widget {
  fn widget(&self) -> &BoxWidget;
  fn widget_mut(&mut self) -> &mut BoxWidget;

  #[inline]
  fn reattach(&mut self, widget: BoxWidget) -> BoxWidget {
    std::mem::replace(self.widget_mut(), widget)
  }
}

pub trait HarvestAttr {
  type W: Widget;
  fn attach_attr<AttrData: Default + 'static>(self) -> WidgetAttr<Self::W, AttrData>;
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
      marker: PhantomData,
    };
    erase_type.box_it()
  }
}

fn copy_split_attr<AttrData: 'static>(widget: &mut BoxWidget) -> Option<(BoxWidget, AttrData)> {
  if let Some(attr) = widget
    .as_any_mut()
    .downcast_mut::<WidgetAttr<BoxWidget, AttrData>>()
  {
    let mut tmp = std::mem::MaybeUninit::uninit();
    let ptr: *mut WidgetAttr<BoxWidget, AttrData> = tmp.as_mut_ptr();
    let tmp = unsafe {
      ptr.copy_from(
        attr as *const WidgetAttr<BoxWidget, AttrData>,
        std::mem::size_of::<WidgetAttr<BoxWidget, AttrData>>(),
      );
      tmp.assume_init()
    };
    Some((tmp.widget, tmp.attr))
  } else {
    None
  }
}

impl<W: Widget> HarvestAttr for W {
  default type W = BoxWidget;
  default fn attach_attr<AttrData: Default + 'static>(self) -> WidgetAttr<Self::W, AttrData> {
    let mut boxed = self.box_it();
    if let Some((widget, attr)) = copy_split_attr(&mut boxed) {
      std::mem::forget(boxed);
      WidgetAttr {
        attr,
        widget,
        marker: PhantomData,
      }
    } else {
      let mut target = boxed.as_attr_mut();
      let mut attr = None;
      while let Some(attr_widget) = target.take() {
        if let Some((widget, a)) = copy_split_attr(attr_widget.widget_mut()) {
          let detached = attr_widget.reattach(widget);
          std::mem::forget(detached);
          attr = Some(a);
          break;
        } else {
          target = attr_widget.widget_mut().as_attr_mut();
        }
      }

      let attr = attr.unwrap_or_else(|| unimplemented!());
      WidgetAttr {
        attr,
        widget: boxed,
        marker: PhantomData,
      }
    }
  }
}

impl<W: Widget, Data: Debug + 'static> HarvestAttr for WidgetAttr<W, Data> {
  type W = W;
}
