#![crate_name = "asmrt"]
#![crate_type = "lib"]
#![no_std]

#![feature(lang_items)]

#![allow(unstable)] // much of libcore is unstable as of Rust 1.0

#[macro_use] extern crate core;

use core::prelude::*;
use core::fmt;


mod std {
    pub use core::fmt;
}


// Essential lang items.  These would normally be provided by librustrt.

#[inline(always)] #[cold]
#[lang = "panic_fmt"]
extern fn lang_panic_fmt(args: &core::fmt::Arguments,
                        file: &'static str,
                        line: usize) -> ! {
    raw_println(format_args!("task panicked at {}:{}: {}", file, line, args));
    unsafe { core::intrinsics::abort() };
}

#[inline(always)] #[cold]
#[lang = "stack_exhausted"]
extern fn lang_stack_exhausted() -> ! {
    unsafe {
        let s = "task panicked - stack exhausted";
        write_str(s.as_ptr(), s.len() as i32);
        flush_str();
    }
    unsafe { core::intrinsics::abort() };
}

#[inline(always)] #[cold]
#[lang = "eh_personality"]
extern fn lang_eh_personality() -> ! {
    unsafe { core::intrinsics::abort() };
}


// Implementation of `println!`

extern {
    fn write_str(data: *const u8, len: i32);
    fn flush_str();
}

struct AsmJsFormatWriter;

impl fmt::Writer for AsmJsFormatWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe { write_str(s.as_ptr(), s.len() as i32) };
        Ok(())
    }
}

pub fn raw_println(args: fmt::Arguments) {
    let _ = fmt::write(&mut AsmJsFormatWriter, args);
    unsafe { flush_str() };
}

#[macro_export]
macro_rules! println {
    ($str:expr) => {
        $crate::raw_println(format_args!($str))
    };
    ($str:expr, $($rest:tt)*) => {
        $crate::raw_println(format_args!($str, $($rest)*))
    };
}

#[macro_export]
macro_rules! log {
    ($level:expr, $str:expr) => {
        println!($str)
    };
    ($level:expr, $str:expr, $($rest:tt)*) => {
        println!($str, $($rest)*)
    };
}


// Generic interface for calling back into Javascript code.

pub fn run_callback(idx: i32, args: &[i32]) -> i32 {
    extern {
        fn run_callback(idx: i32, arg_base: *const i32, arg_len: i32) -> i32;
    }
    unsafe { run_callback(idx, args.as_ptr(), args.len() as i32) }
}
