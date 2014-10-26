RUSTC ?= rustc
PYTHON ?= python
PYTHON3 ?= python3
RUST_SRC ?= ../rust
EM_FASTCOMP ?= /usr
EM_PLUGINS ?= 
CLOSURE_COMPILER ?= closure-compiler

HOST = x86_64-unknown-linux-gnu
TARGET = i686-unknown-linux-gnu

ASMJS_OUT = build/asmjs
NATIVE_OUT = build/native
ASMLIBS_OUT = build/asmlibs
MIN_OUT = build/min
DIST = dist
$(shell mkdir -p $(ASMJS_OUT) $(NATIVE_OUT) $(ASMLIBS_OUT) $(MIN_OUT) $(DIST))


JS_SRCS = $(wildcard client/js/*.js)


all: $(DIST)/all


# Dependencies of Rust libraries

define LIB_DEPS
build/$(1)/lib$(2).rlib: $(foreach dep,$(3),build/$(1)/lib$(dep).rlib)
build/$(1)/lib$(2).so: $(foreach dep,$(3),build/$(1)/lib$(dep).so)
endef

DEPS_physics_asmjs = core asmrt
DEPS_physics_native =
DEPS_graphics_asmjs = core asmrt physics
DEPS_graphics_native = physics
ALL_LIBS = asmrt physics graphics

$(foreach mode,asmjs native, \
 $(eval $(foreach lib,$(ALL_LIBS), \
  $(eval $(call LIB_DEPS,$(mode),$(lib),$(DEPS_$(lib)_$(mode)))))))


# Dependencies of client/asmlibs.rs

DEPS_asmlibs = core asmrt physics graphics


# Rules for building Rust libraries

RUSTFLAGS_asmjs = -L $(ASMJS_OUT) -L $(NATIVE_OUT) \
		--opt-level=3 --target=$(TARGET) \
		-Z no-landing-pads -C no-stack-check --cfg asmjs

RUSTFLAGS_native = -L $(NATIVE_OUT) \
		--opt-level=3 --target=$(HOST)

$(ASMJS_OUT)/lib%.rlib: %/lib.rs
	$(RUSTC) $< --out-dir $(ASMJS_OUT) --crate-type=rlib $(RUSTFLAGS_asmjs) \
		--dep-info $(ASMJS_OUT)/$*.d

# Special rule for libcore, since its source code is in a weird location.
$(ASMJS_OUT)/libcore.rlib: $(RUST_SRC)/src/libcore/lib.rs
	$(RUSTC) $< --out-dir $(ASMJS_OUT) --crate-type=rlib $(RUSTFLAGS_asmjs)

$(NATIVE_OUT)/lib%.rlib: %/lib.rs
	$(RUSTC) $< --out-dir $(NATIVE_OUT) --crate-type=rlib $(RUSTFLAGS_native) \
		--dep-info $(NATIVE_OUT)/$*.d

$(NATIVE_OUT)/lib%.so: %/lib.rs
	$(RUSTC) $< --out-dir $(NATIVE_OUT) --crate-type=dylib $(RUSTFLAGS_native) \
		--dep-info $(NATIVE_OUT)/$*.d

-include $(wildcard $(ASMJS_OUT)/*.d $(NATIVE_OUT)/*.d)


# Rules for building asmlibs.js

ASMLIBS = $(ASMLIBS_OUT)/asmlibs

$(ASMLIBS).ll: client/asmlibs.rs $(foreach dep,$(DEPS_asmlibs),$(ASMJS_OUT)/lib$(dep).rlib)
	$(RUSTC) $< -o $@ --emit=ir --crate-type=staticlib $(RUSTFLAGS_asmjs) -C lto

$(ASMLIBS).clean.ll: $(ASMLIBS).ll
	sed -e 's/\<\(readonly\|readnone\|cold\)\>//g' \
		-e 's/\<dereferenceable([0-9]*)//g' \
		$< >$@

$(ASMLIBS).bc: $(ASMLIBS).clean.ll
	$(EM_FASTCOMP)/bin/llvm-as $< -o $@

$(ASMLIBS).opt.bc: $(ASMLIBS).bc client/asmlibs_exports.txt
	$(EM_FASTCOMP)/bin/opt $< \
		-load=$(EM_PLUGINS)/BreakStructArguments.so \
		-strip-debug \
		-internalize -internalize-public-api-list=$(shell tr '\n' ',' <client/asmlibs_exports.txt) \
		-break-struct-arguments \
		-globaldce \
		-pnacl-abi-simplify-preopt -pnacl-abi-simplify-postopt \
		-enable-emscripten-cxx-exceptions \
		-o $@

$(ASMLIBS).0.js: $(ASMLIBS).opt.bc
	$(EM_FASTCOMP)/bin/llc $< \
		-march=js -filetype=asm \
		-emscripten-assertions=1 \
		-emscripten-no-aliasing-function-pointers \
		-emscripten-max-setjmps=20 \
		-O3 \
		-o $@

$(ASMLIBS).1.js: $(ASMLIBS).0.js util/asmjs_function_tables.py
	$(PYTHON) util/asmjs_function_tables.py <$< >$@

$(ASMLIBS).js: client/asmlibs.tmpl.js $(ASMLIBS).1.js util/asmjs_insert_functions.awk
	awk -f util/asmjs_insert_functions.awk <$< >$@


# Rules for running closure compiler

CLOSURE_FLAGS=--language_in=ECMASCRIPT5_STRICT \
			  --output_wrapper='(function(){%output%})();'

$(MIN_OUT)/asmlibs.js: $(ASMLIBS_OUT)/asmlibs.js
	$(CLOSURE_COMPILER) $(CLOSURE_FLAGS) \
		$< --js_output_file=$@ --compilation_level=WHITESPACE_ONLY

$(MIN_OUT)/outpost.js: $(JS_SRCS)
	$(CLOSURE_COMPILER) $(CLOSURE_FLAGS) \
		$^ --js_output_file=$@ --compilation_level=ADVANCED_OPTIMIZATIONS \
		--process_common_js_modules --common_js_entry_module=main \
		--common_js_module_path_prefix=client/js/ --externs=util/closure_externs.js


# Rules for misc files

build/tiles.json: client/assets/tiles.yaml util/make_tiles_json.py
	$(PYTHON3) util/make_tiles_json.py <$< >$@

build/client.debug.html: client/client.html \
	util/collect_js_deps.py util/patch_script_tags.py $(JS_SRCS)
	$(PYTHON3) util/collect_js_deps.py client/js/main.js | \
		$(PYTHON3) util/patch_script_tags.py $< >$@


# Rules for copying files into dist/

define DIST_FILE_
$(DIST)/$(1): $(2)
	cp -v $$< $$@

$(DIST)/all: $(DIST)/$(1)
endef
DIST_FILE = $(call DIST_FILE_,$(strip $(1)),$(strip $(2)))

$(eval $(call DIST_FILE, tiles.json, 	build/tiles.json))

ifeq ($(RELEASE),)
$(eval $(call DIST_FILE, client.html, 	build/client.debug.html))
$(eval $(call DIST_FILE, shim.js, 		client/shim.js))
$(eval $(call DIST_FILE, asmlibs.js, 	build/asmlibs/asmlibs.js))
dist/all: $(patsubst client/js/%,dist/js/%,$(JS_SRCS))

$(shell mkdir -p $(DIST)/js)
else
$(eval $(call DIST_FILE, client.html, 	client/client.html))
$(eval $(call DIST_FILE, outpost.js, 	build/min/outpost.js))
$(eval $(call DIST_FILE, asmlibs.js, 	build/min/asmlibs.js))
endif


$(DIST)/assets/%: client/assets/%
	mkdir -p $$(dirname $@)
	cp -v $< $@

$(DIST)/js/%: client/js/%
	cp -v $< $@

$(DIST)/all:

.PHONY: all $(DIST)/all
