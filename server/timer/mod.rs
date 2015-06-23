use std::boxed::FnBox;
use std::mem;
use std::sync::mpsc::Receiver;

use types::*;

use engine::split::EngineRef;

pub use self::queue::Cookie;
pub use self::queue::WakeQueue;


mod queue;

pub struct Timer {
    queue: WakeQueue<Box<FnBox(EngineRef)+'static>>,
    time_base: Time,
}

pub struct TimerEvent(Cookie);

fn cast_receiver(recv: &Receiver<Cookie>) -> &Receiver<TimerEvent> {
    unsafe { mem::transmute(recv) }
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            queue: WakeQueue::new(),
            time_base: 0,
        }
    }


    // Keep track of the delta between world time and UTC.  The WakeQueue operates on UTC
    // exclusively, while the rest of the system uses world time, so we have to convent back and
    // forth in this module.

    fn world_time(&self, unix_time: Time) -> Time {
        unix_time - self.time_base
    }

    // NB: This is designed to be called only once, near the beginning of server startup.  Calling
    // it while the server is running may have strange effects.
    pub fn set_world_time(&mut self, unix_time: Time, world_time: Time) {
        self.time_base = unix_time - world_time;
        debug!("new time_base: {:x} (world_time {:x})", self.time_base, world_time);
    }

    fn from_world_time(&self, world_time: Time) -> Time {
        world_time + self.time_base
    }


    pub fn schedule<F>(&mut self, when: Time, cb: F) -> Cookie
            where F: FnOnce(EngineRef)+'static {
        let unix_when = self.from_world_time(when);
        self.queue.schedule(unix_when, Box::new(cb))
    }

    pub fn cancel(&mut self, cookie: Cookie) {
        self.queue.cancel(cookie);
    }

    pub fn receiver(&self) -> &Receiver<TimerEvent> {
        cast_receiver(self.queue.receiver())
    }

    pub fn process(&mut self, evt: TimerEvent) -> Option<(Box<FnBox(EngineRef)+'static>, Time)> {
        self.queue.retrieve(evt.0)
            .map(|(unix_when, cb)| (cb, self.world_time(unix_when)))
    }
}
