use std::mem;
use std::ptr;
use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use std::thread;
use std::u64;

use types::Time;
use util::now;
use util::SmallVec;
use util::IdMap;


const BUCKET_BITS: usize = 3;
const BUCKET_MS: u32 = 1 << BUCKET_BITS;
const COOKIE_BITS: usize = 32 - BUCKET_BITS;

const WHEEL_BITS: usize = 17;
const WHEEL_MS: u32 = 1 << WHEEL_BITS;
const WHEEL_BUCKETS: usize = 1 << (WHEEL_BITS - BUCKET_BITS);

const UPDATE_INTERVAL: u32 = WHEEL_MS / 2;



#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct WakeSoon(u32);

impl WakeSoon {
    fn new(offset: u32, cookie: u32) -> WakeSoon {
        assert!(offset < (1 << BUCKET_BITS));
        assert!(cookie < (1 << COOKIE_BITS));

        WakeSoon(offset | (cookie << BUCKET_BITS))
    }

    fn offset(self) -> u32 {
        self.0 & ((1 << BUCKET_BITS) - 1)
    }

    fn cookie(self) -> u32 {
        self.0 >> BUCKET_BITS
    }
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Wake {
    when: Time,
    cookie: u32,
}

impl Wake {
    fn new(when: Time, cookie: u32) -> Wake {
        Wake {
            when: when,
            cookie: cookie,
        }
    }

    fn when(self) -> Time {
        self.when
    }

    fn cookie(self) -> u32 {
        self.cookie
    }
}


struct Wheel {
    now: Time,
    soon: Box<[SmallVec<WakeSoon>; WHEEL_BUCKETS]>,
    later: Vec<Wake>,
}

impl Wheel {
    pub fn new(now: Time) -> Wheel {
        // Can't set the size of the array using `size_of`, so hardcode the value and then check at
        // runtime that it's right.
        assert!(mem::size_of::<SmallVec<WakeSoon>>() == 4 * u64::BYTES as usize);
        let fake_smallvec = [0_u64; 4];
        let mut soon = Box::new([fake_smallvec; WHEEL_BUCKETS]);
        unsafe {
            let soon_view: &mut [_; WHEEL_BUCKETS] = mem::transmute(&mut *soon);
            for r in soon_view.iter_mut() {
                ptr::write(r, SmallVec::<WakeSoon>::new());
            }
        }

        Wheel {
            now: now,
            soon: unsafe { mem::transmute(soon) },
            later: Vec::new(),
        }
    }

    pub fn schedule(&mut self, wake: Wake) {
        let Wake { when, cookie } = wake;
        let when = if when < self.now { self.now } else { when };
        if when - self.now >= WHEEL_MS as Time {
            self.later.push(Wake::new(when, cookie));
        } else {
            let bucket_idx = (when as u32 & (WHEEL_MS - 1)) as usize >> BUCKET_BITS;
            let offset = when as u32 & (BUCKET_MS - 1);
            self.soon[bucket_idx].push(WakeSoon::new(offset, cookie));
        }
    }

    pub fn cancel(&mut self, wake: Wake) -> bool {
        let Wake { when, cookie } = wake;
        if when < self.now {
            // Too late - it already fired.
            return false;
        }

        // Check the normal bucket first.
        let bucket_idx = (when as u32 & (WHEEL_MS - 1)) as usize >> BUCKET_BITS;
        let offset = when as u32 & (BUCKET_MS - 1);
        {
            let mut bucket = &mut self.soon[bucket_idx];
            let wake = WakeSoon::new(offset, cookie);
            for i in 0 .. bucket.len() {
                if bucket[i] == wake {
                    bucket.swap_remove(i);
                    return true;
                }
            }
        }

        // Wasn't found in the normal bucket.  Check the `later` bucket as well.
        {
            for i in 0 .. self.later.len() {
                if self.later[i] == wake {
                    self.later.swap_remove(i);
                    return true;
                }
            }
        }

        // Wasn't found at all.  Oh well.
        return false;
    }

    pub fn advance(&mut self) -> SmallVec<WakeSoon> {
        let bucket_idx = (self.now as u32 & (WHEEL_MS - 1)) as usize >> BUCKET_BITS;

        let mut bucket = mem::replace(&mut self.soon[bucket_idx], SmallVec::new());
        bucket.sort_by(|a, b| a.offset().cmp(&b.offset()));
        let bucket = bucket;

        self.now += BUCKET_MS as Time;

        if self.now % UPDATE_INTERVAL as Time == 0 {
            // Find any elements in `self.later` that need to be moved into `self.soon`.
            let mut i = 0;
            while i < self.later.len() {
                if self.later[i].when < self.now + WHEEL_MS as Time {
                    let wake = self.later.swap_remove(i);
                    self.schedule(wake);
                    // Don't increment - process the element that was just swapped into index `i`.
                } else {
                    i += 1;
                }
            }
        }

        bucket
    }

    pub fn now(&self) -> Time {
        self.now
    }

    pub fn next_tick(&self) -> Time {
        self.now + BUCKET_MS as Time
    }
}



enum Command {
    Schedule(Wake),
    Cancel(Wake),
}

fn timer_worker(recv: Receiver<Command>, send: Sender<Cookie>) {
    let mut wheel = Wheel::new(now() & !(BUCKET_MS as Time - 1));
    loop {
        // `wheel.now` lags behind `now()` by up to `BUCKET_MS`.
        //
        // wheel.now v
        //   |-------|-------|-------|
        //          now() ^
        //
        // This means that it's possible to schedule a wakeup at time `now()` and have it actually
        // fire at that (logical) time - it doesn't need to be delayed until the next wheel tick.
        //
        // wheel.now v
        //   |-------|----O--|-------|
        //          now() ^
        //
        // Once `now` reaches the next wheel tick, we run `wheel.advance()` to catch it up.
        //
        //         wheel.now v
        //   |-------|----O--|-------|
        //             now() ^
        //
        // This processes the wakeup.
        //
        // Note that if one wakeup tries to schedule another wakeup, the second wakeup *will* be
        // delayed, since `wheel.now` has already advanced to the next tick.

        // Wait until the next tick.
        loop {
            let delay = wheel.next_tick() - now();
            if delay > 0 {
                thread::sleep_ms(delay as u32);
            } else {
                break;
            }
        }

        // Flush the receive queue just before advancing the wheel.  This ensures we pick up every
        // request that was sent before the new tick.
        loop {
            match recv.try_recv() {
                Ok(Command::Schedule(wake)) => wheel.schedule(wake),
                Ok(Command::Cancel(wake)) => {
                    if wheel.cancel(wake) {
                        send.send(Cookie(wake.cookie())).unwrap();
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    warn!("timer_worker: recv disconnected; exiting...");
                    return;
                },
            }
        }

        // Advance the wheel, sending wakeup events back to the owner.
        let wakes = wheel.advance();
        for wake in wakes.into_iter() {
            send.send(Cookie(wake.cookie())).unwrap();
        }
    }
}



/// Wrapper struct for timer cookies.  This is non-Copy to prevent obviously wrong behavior, but
/// it's still possible to get two copies of the same Cookie (one from `schedule` and one from the
/// `receiver()`).
#[derive(Debug)]
pub struct Cookie(u32);


#[derive(Debug)]
struct WakeItem<T> {
    time: Time,
    cancelled: bool,
    reason: T,
}

impl<T> WakeItem<T> {
    fn new(time: Time, reason: T) -> WakeItem<T> {
        WakeItem {
            time: time,
            cancelled: false,
            reason: reason,
        }
    }
}


pub struct WakeQueue<T> {
    send: Sender<Command>,
    recv: Receiver<Cookie>,
    items: IdMap<WakeItem<T>>,
}

impl<T> WakeQueue<T> {
    pub fn new() -> WakeQueue<T> {
        let (send_cmd, recv_cmd) = channel();
        let (send_wake, recv_wake) = channel();

        thread::spawn(|| {
            timer_worker(recv_cmd, send_wake);
        });

        WakeQueue {
            send: send_cmd,
            recv: recv_wake,
            items: IdMap::new(),
        }
    }

    pub fn schedule(&mut self, when: Time, reason: T) -> Cookie {
        let raw_cookie = self.items.insert(WakeItem::new(when, reason));
        assert!(raw_cookie < (1 << COOKIE_BITS));
        self.send.send(Command::Schedule(Wake::new(when, raw_cookie as u32))).unwrap();
        Cookie(raw_cookie as u32)
    }

    pub fn cancel(&mut self, cookie: Cookie) {
        // Might have already been retrieved, since it's possible to get two duplicate Cookie
        // values.
        if let Some(item) = self.items.get_mut(cookie.0 as usize) {
            self.send.send(Command::Cancel(Wake::new(item.time, cookie.0))).unwrap();
            item.cancelled = true;
        }
    }

    pub fn receiver(&self) -> &Receiver<Cookie> {
        &self.recv
    }

    /// Retrieve the data associated with a fired event cookie.  Returns `None` if the timer has
    /// already been cancelled, for example, if the timer was cancelled while its Cookie was
    /// already waiting in the Receiver queue.
    pub fn retrieve(&mut self, cookie: Cookie) -> Option<(Time, T)> {
        if let Some(item) = self.items.remove(cookie.0 as usize) {
            if !item.cancelled {
                return Some((item.time, item.reason));
            }
        }
        None
    }
}
