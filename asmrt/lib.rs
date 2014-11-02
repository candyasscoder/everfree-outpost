#![crate_name = "asmrt"]
#![crate_type = "lib"]
#![no_std]

#![feature(globs, phase)]
#![feature(lang_items)]
#![feature(macro_rules)]

#[phase(plugin, link)] extern crate core;

use core::prelude::*;
use core::fmt;


mod std {
    pub use core::fmt;
}


// Essential lang items.  These would normally be provided by librustrt.

#[inline(always)] #[cold]
#[lang = "fail_fmt"]
extern fn lang_fail_fmt(args: &core::fmt::Arguments,
                        file: &'static str,
                        line: uint) -> ! {
    format_args!(raw_println, "task failed at {}:{}: {}", file, line, args);
    unsafe { core::intrinsics::abort() };
}

#[inline(always)] #[cold]
#[lang = "stack_exhausted"]
extern fn lang_stack_exhausted() -> ! {
    unsafe {
        let s = "task failed - stack exhausted";
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

impl fmt::FormatWriter for AsmJsFormatWriter {
    fn write(&mut self, bytes: &[u8]) -> fmt::Result {
        unsafe { write_str(bytes.as_ptr(), bytes.len() as i32) };
        Ok(())
    }
}

pub fn raw_println(args: &fmt::Arguments) {
    let _ = fmt::write(&mut AsmJsFormatWriter, args);
    unsafe { flush_str() };
}

#[macro_export]
macro_rules! println {
    ($str:expr $($rest:tt)*) => {
        format_args!(::asmrt::raw_println, $str $($rest)*)
    }
}
