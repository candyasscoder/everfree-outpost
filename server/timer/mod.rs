use std::boxed::FnBox;
use std::sync::mpsc::Receiver;

use types::*;

use engine::split::EngineRef;

pub use self::queue::Cookie;
pub use self::queue::WakeQueue;


mod queue;

pub struct Timer {
    queue: WakeQueue<Box<FnBox(EngineRef, Time)+'static>>,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            queue: WakeQueue::new(),
        }
    }

    pub fn schedule<F>(&mut self, when: Time, cb: F) -> Cookie
            where F: FnOnce(EngineRef, Time)+'static {
        self.queue.schedule(when, Box::new(cb))
    }

    pub fn cancel(&mut self, cookie: Cookie) {
        self.queue.cancel(cookie);
    }

    pub fn receiver(&self) -> &Receiver<Cookie> {
        self.queue.receiver()
    }

    pub fn process(mut eng: EngineRef, cookie: Cookie) {
        let (when, cb) = eng.timer_mut().queue.retrieve(cookie);
        // `cb(eng, when)` does not compile, see rust-lang/rust #25647
        cb.call_box((eng, when));
    }
}
