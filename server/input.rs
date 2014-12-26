#![allow(dead_code)]

bitflags! {
    flags InputBits: u16 {
        const INPUT_LEFT =      0x0001,
        const INPUT_RIGHT =     0x0002,
        const INPUT_UP =        0x0004,
        const INPUT_DOWN =      0x0008,
        const INPUT_RUN =       0x0010,
    }
}

bitflags! {
    flags ActionBits: u16 {
        const ACT_USE =         0x0001,
    }
}
