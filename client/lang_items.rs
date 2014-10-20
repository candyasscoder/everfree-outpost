use core;

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
