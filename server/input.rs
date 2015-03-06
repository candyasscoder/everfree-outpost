#![allow(dead_code)]
use types::*;


bitflags! {
    flags InputBits: u16 {
        const INPUT_LEFT =      0x0001,
        const INPUT_RIGHT =     0x0002,
        const INPUT_UP =        0x0004,
        const INPUT_DOWN =      0x0008,
        const INPUT_RUN =       0x0010,
    }
}


#[derive(Copy, PartialEq, Eq, Debug)]
pub struct ActionId(pub u16);

macro_rules! action_ids {
    ($($name:ident = $val:expr,)*) => {
        $( pub const $name: ActionId = ActionId($val); )*
    }
}

action_ids! {
    ACTION_USE =        1,
    ACTION_INVENTORY =  2,
    ACTION_USE_ITEM =   3,
}


#[derive(Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Use,
    Inventory,
    UseItem(ItemId),
}

impl Action {
    pub fn decode(action: u16, arg: u32) -> Option<Action> {
        match (action, arg) {
            (1, 0) => Some(Action::Use),
            (2, 0) => Some(Action::Inventory),
            (3, _) => Some(Action::UseItem(arg as ItemId)),
            _ => None,
        }
    }
}
