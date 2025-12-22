cfg_if::cfg_if! {
  if #[cfg(target_arch = "wasm32")] {
    mod imp {
      use std::future::Future;

      pub(crate) struct UiExecutor;

      impl UiExecutor {
        pub(crate) fn new(
          _: winit::event_loop::EventLoopProxy<super::super::RibirAppEvent>,
        ) -> Self {
          Self
        }

        pub(crate) fn spawn_local(&self, fut: impl Future<Output = ()> + 'static) {
          wasm_bindgen_futures::spawn_local(fut);
        }

        pub(crate) fn pump(&self) {}
      }
    }
  } else {
    mod imp {
      use std::{
        future::Future,
        pin::Pin,
        sync::Arc,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Waker},
      };

      use pin_project_lite::pin_project;
      use tokio::task::LocalSet;
      use winit::event_loop::EventLoopProxy;

      use super::super::{AppEvent, RibirAppEvent};

      pub(crate) struct UiExecutor {
        rt: tokio::runtime::Runtime,
        local_set: LocalSet,
        event_loop_proxy: EventLoopProxy<RibirAppEvent>,
        wake_posted: Arc<AtomicBool>,
      }

      impl UiExecutor {
        pub(crate) fn new(event_loop_proxy: EventLoopProxy<RibirAppEvent>) -> Self {
          let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("Failed building UI Runtime");

          Self {
            rt,
            local_set: LocalSet::new(),
            event_loop_proxy,
            wake_posted: Arc::new(AtomicBool::new(false)),
          }
        }

        pub(crate) fn spawn_local(&self, fut: impl Future<Output = ()> + 'static) {
          let wrapped = WakeOnPoll {
            fut,
            event_loop_proxy: self.event_loop_proxy.clone(),
            wake_posted: self.wake_posted.clone(),
          };
          self.local_set.spawn_local(wrapped);
        }

        pub(crate) fn pump(&self) {
          // Allow a new wake event to be posted after each pump cycle.
          self.wake_posted.store(false, Ordering::Release);

          // Drive one turn of the UI executor.
          // `yield_now()` guarantees at least one scheduler yield, and `LocalSet::block_on`
          // makes progress on all ready local tasks while waiting for it.
          self.local_set.block_on(&self.rt, async {
            tokio::task::yield_now().await;
          });
        }
      }

      pin_project! {
        struct WakeOnPoll<F> {
          #[pin]
          fut: F,
          event_loop_proxy: EventLoopProxy<RibirAppEvent>,
          wake_posted: Arc<AtomicBool>,
        }
      }

      impl<F> WakeOnPoll<F>
      where
        F: Future,
      {
        fn composite_waker(&self, cx: &Context<'_>) -> Waker {
          struct UiWake {
            waker: Waker,
            event_loop_proxy: EventLoopProxy<RibirAppEvent>,
            wake_posted: Arc<AtomicBool>,
          }

          impl std::task::Wake for UiWake {
            fn wake(self: Arc<Self>) {
              self.wake_by_ref();
            }

            fn wake_by_ref(self: &Arc<Self>) {
              self.waker.wake_by_ref();

              // Coalesce repeated wakes into a single winit user-event.
              if self.wake_posted.swap(true, Ordering::AcqRel) {
                return;
              }
              let _ = self
                .event_loop_proxy
                .send_event(RibirAppEvent::App(AppEvent::FuturesWake));
            }
          }

          let composite = Arc::new(UiWake {
            waker: cx.waker().clone(),
            event_loop_proxy: self.event_loop_proxy.clone(),
            wake_posted: self.wake_posted.clone(),
          });
          Waker::from(composite)
        }
      }

      impl<F> Future for WakeOnPoll<F>
      where
        F: Future,
      {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
          let waker = self.composite_waker(cx);
          let mut cx = Context::from_waker(&waker);
          let this = self.project();
          this.fut.poll(&mut cx)
        }
      }
    }
  }
}

pub(crate) use imp::UiExecutor;
