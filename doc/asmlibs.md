The client-side Javascript code comes in two parts.  The code in `outpost.js`
(in the release build) is ordinary Javascript found under `client/js/`.  The
code in `asmlibs.js` consists of several Rust components that get compiled to
Javascript using the Emscripten LLVM backend.  The `outpost.js` component
contains most of the client logic, while `asmlibs.js` contains especially
performance-critical code (currently physics and graphics calculations).

Building the `asmlibs.js` library involves several distinct pieces:

 * The actual Rust libraries.  Currently these are `physics` and `graphics`.
   (Note that this is the exact same physics library used by the server.)
   These libraries are carefully written to use only `libcore` (at least when
   built under `#[cfg(asmjs)]`).  In particular, this means the library cannot
   perform dynamic allocation.  Any variable-length buffers must be provided by
   the calling code.

 * The `client/asmlibs.rs` library.  This is glue code that defines `extern fn`
   wrappers around the main libraries with asm.js-friendly arguments.

 * The `asmrt` library.  This contains the Rust side of essential low-level
   infrastructure, including required `lang_items` and the `println!` macro
   (for debugging).

 * The template `client/asmlibs.tmpl.js`.  This contains the wrapper code that
   defines parts of the essential asm.js infrastructure, such as the `HEAP`
   arrays and the asm.js implementation of `memcpy` (since asmlibs does not use
   an Emscripten-compiled libc).  Other infrastructure (particularly the
   Javascript code backing the definitions in `asmrt`) is taken from the
   environment used to construct the asm.js module.  The template contains
   placeholders for code and static data that get filled in by
   `util/asmjs_insert_functions.awk`.

 * `client/asmlibs_exports.txt` lists symbols that should be considered
   exported by asmlibs.  This information is (unfortunately) duplicated in the
   export object produced in `asmlibs.tmpl.js`.

 * The `client/js/asmlibs.js` (actually a component of `outpost.js`) defines
   the high-level interface between `asmlibs.js` and the rest of `outpost.js`.
   It provides the `Asm` class, which wraps the raw asm.js module (providing an
   appropriate environment, with the Javascript implementations of `asmrt`
   functions) and handles marshalling/unmarshalling of arguments and return
   values.  This library also handles address/size calculations to compute
   offsets of various buffers and appropriate heap sizes for the asm.js module.


## Adding a new function

 1. Add the actual function to the `physics`/`graphics` library.  This should
    be written like any ordinary Rust function, except that it should only rely
    on `libcore`.

 2. Add the Rust wrapper function to `client/asmlibs.rs`.  This function should
    be `extern` and should follow a restricted ABI: use only integer (32 bit or
    less, no `u64`/`i64`) and pointer types for arguments and return types.
    Use an explicit "out pointer" if the function needs to return multiple
    values.  The function must have an `#[export_name = "my_function"]`
    attribute.

 3. Add the symbol name (from the `export_name` attribute) to
    `client/asmlibs_exports.txt`.  Also add a corresponding line to the export
    object in `client/asmlibs.tmpl.js`.  The export line should have an
    underscore prefix on the right, but not on the left, as in `my_function:
    _my_function`.  (The Emscripten LLVM backend adds this prefix in generated
    code.)

 4. Add a Javascript wrapper function to `Asm.prototype` in
    `client/js/asmlibs.js`.  For (Rust) integer arguments, the wrapper can just
    pass the Javascript number.  For complex arguments such as `V3`s, or to
    obtain a buffer to pass as the out pointer, it should use
    `this._stackAlloc` to obtain some scratch space on the asm.js stack.
    (Don't forget to `this._stackFree` when done, in reverse order from the
    allocation.)  For large fixed buffers (such as the geometry buffers filled in by
    the graphics library), use the `XYZ_START` constants.
