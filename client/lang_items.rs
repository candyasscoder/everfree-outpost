#![macro_escape]

use core::prelude::*;
use core;
use core::fmt;
use core::fmt::FormatWriter;

extern {
    fn write_str(data: *const u8, len: i32);
    fn flush_str();
}

struct AsmJsFormatWriter;

impl FormatWriter for AsmJsFormatWriter {
    fn write(&mut self, bytes: &[u8]) -> fmt::Result {
        unsafe { write_str(bytes.as_ptr(), bytes.len() as i32) };
        Ok(())
    }
}

pub fn println(args: &fmt::Arguments) {
    fmt::write(&mut AsmJsFormatWriter, args);
    unsafe { flush_str() };
}

macro_rules! println {
    ($str:expr $($rest:tt)*) => {
        format_args!(::lang_items::println, $str $($rest)*)
    }
}


#[inline(always)] #[cold]
#[lang = "fail_fmt"]
extern fn lang_fail_fmt(args: &core::fmt::Arguments,
                        file: &'static str,
                        line: uint) -> ! {
    unsafe { core::intrinsics::abort() };
}

#[inline(always)] #[cold]
#[lang = "stack_exhausted"]
extern fn lang_stack_exhausted() -> ! {
    unsafe { core::intrinsics::abort() };
}

#[inline(always)] #[cold]
#[lang = "eh_personality"]
extern fn lang_eh_personality() -> ! {
    unsafe { core::intrinsics::abort() };
}
