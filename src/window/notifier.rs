use webrender::api::{DocumentId, RenderNotifier};
use winit::event_loop::EventLoopProxy;

pub struct Notifier {
    events_proxy: EventLoopProxy<()>,
}

impl Notifier {
    pub fn new(events_proxy: EventLoopProxy<()>) -> Notifier {
        Notifier { events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self) {
        self.events_proxy.send_event(()).ok();
    }

    fn new_frame_ready(
        &self,
        _: DocumentId,
        _scrolled: bool,
        _composite_needed: bool,
        _render_time_ns: Option<u64>,
    ) {
        self.wake_up();
    }
}
