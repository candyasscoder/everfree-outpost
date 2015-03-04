use std::sync::mpsc::{Sender, Receiver};

use types::*;
use input::InputBits;
use input::ActionId;
use msg::{Request, Response};
use timer::WakeQueue;

pub struct Events {
    send: Sender<(WireId, Response)>,
    recv: Receiver<(WireId, Request)>,
    wake: WakeQueue<WakeReason>,
}

pub enum Event {
    Request(SenderId, Request),
    Wakeup(ClientId, WakeReason),
}

pub enum SenderId {
    Control,
    Wire(WireId),
    Client(ClientId),
}

#[derive(Copy)]
pub enum WakeReason {
    HandleInput(ClientId, InputBits),
    HandleAction(ClientId, ActionId, u32),
    PhysicsUpdate(EntityId),
    CheckView(ClientId),
}


impl Events {
    pub fn new(recv: Receiver<(WireId, Request)>,
               send: Sender<(WireId, Response)>) -> Events {
        Events {
            send: send,
            recv: recv,
            wake: WakeQueue::new(),
        }
    }

    pub fn next(&mut self) -> (Event, Time) {
        unimplemented!()
    }

    pub fn wire_to_client(&self, wire: WireId) -> Option<ClientId> {
        unimplemented!()
    }

    pub fn send_control(&self, msg: Response) {
        self.send.send((CONTROL_WIRE_ID, msg)).unwrap();
    }
}
