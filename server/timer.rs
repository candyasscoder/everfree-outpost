use std::collections::BinaryHeap;
use std::io::timer::Timer;
use std::num::Bounded;
use std::time::Duration;

use time;

struct WakeItem<T> {
    time: i64,
    reason: T,
}

impl<T> PartialEq for WakeItem<T> {
    fn eq(&self, other: &WakeItem<T>) -> bool {
        self.time == other.time
    }
}

impl<T> Eq for WakeItem<T> { }

impl<T> Ord for WakeItem<T> {
    fn cmp(&self, other: &WakeItem<T>) -> Ordering {
        other.time.cmp(&self.time)
    }
}

impl<T> PartialOrd for WakeItem<T> {
    fn partial_cmp(&self, other: &WakeItem<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct WakeQueue<T> {
    items: BinaryHeap<WakeItem<T>>,
    timer: Timer,
}

impl<T> WakeQueue<T> {
    pub fn new() -> WakeQueue<T> {
        WakeQueue {
            items: BinaryHeap::new(),
            timer: Timer::new().unwrap(),
        }
    }

    pub fn push(&mut self, time: i64, reason: T) {
        self.items.push(WakeItem { time: time, reason: reason });
    }

    pub fn pop(&mut self) -> Option<(i64, T)> {
        match self.items.top() {
            None => return None,
            Some(item) => {
                if item.time > now() {
                    return None;
                }
            },
        }

        let item = self.items.pop().unwrap();
        Some((item.time, item.reason))
    }

    pub fn wait_recv(&mut self) -> Receiver<()> {
        let dur = match self.items.top() {
            None => Bounded::max_value(),
            Some(item) => Duration::milliseconds(item.time - now()),
        };
        self.timer.oneshot(dur)
    }
}

pub fn now() -> i64 {
    let timespec = time::get_time();
    (timespec.sec * 1000) + (timespec.nsec as i64 / 1000000)
}
