import os
import shlex
import subprocess
import sys
import tempfile



def with_ext(path, ext):
    base, _ = os.path.splitext(path)
    return base + '.' + ext

class ConfigError(Exception):
    pass

class Checker(object):
    def __init__(self, i, temp_dir, log_file):
        self.i = i
        self.temp_dir = temp_dir
        self.counter = 0
        self.log_file = log_file
        self.failed = False
        self.desc_map = {}

    # Utility functions
    def file(self, ext):
        name = os.path.join(self.temp_dir, 'tmp%06d.%s' % (self.counter, ext))
        self.counter += 1
        return name

    def write(self, ext, content, mode='w'):
        name = self.file(ext)
        with open(name, mode) as f:
            f.write(content)
        self.log('Created file %s with contents:' % name)
        self.trace(content)
        return name

    def log(self, msg, level='INFO'):
        for line in msg.splitlines():
            self.log_file.write(' [%s] %s\n' % (level.center(4), line))

    def warn(self, msg):
        self.log(msg, level='WARN')

    def err(self, msg):
        self.log(msg, level='ERR')

    def trace(self, msg):
        self.log(msg, level='TRC')

    def out(self, msg, level='MSG'):
        self.log(msg, level=level)
        print(msg)

    def out_skip(self, desc1, key2):
        self.out(' * Skipping check for %s because %s was not found' %
                (desc1, self.desc_map[key2]))

    def run(self, prog, args=[], expect_ret=0):
        if prog is None:
            raise ConfigError('relies on a program that was not found')

        full_args = shlex.split(prog) + args
        self.log('Execute: %r' % full_args)
        self.log_file.flush()
        ret = subprocess.call(full_args,
                stdin=subprocess.DEVNULL, stdout=self.log_file, stderr=subprocess.STDOUT)
        self.log_file.flush()
        if expect_ret is None or ret == expect_ret:
            self.log('Process %r returned %d (ok)' % (full_args[0], ret))
        else:
            self.warn('Process %r returned %d (expected %d)' % (full_args[0], ret, expect_ret),)
            raise ConfigError('running %r failed: return code %d (expected %d)' %
                    (full_args[0], ret, expect_ret))

    def run_output(self, prog, args=[], expect_ret=0):
        full_args = shlex.split(prog) + args
        self.log('Execute: %r' % full_args)
        p = subprocess.Popen(full_args,
                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        output, _ = p.communicate()

        if expect_ret is not None and p.returncode != expect_ret:
            raise ConfigError('process %r returned %d (expected %d)' %
                    (full_args[0], p.returncode, expect_ret))

        output = output.decode()
        self.log_file.write(output)
        # If it returns nonzero, we get an exception from check_output.
        self.log('Process %r returned %d (ok)' % (full_args[0], p.returncode))
        return output

    # Top-level wrappers for checks
    def do_check(self, check, *args):
        self.log('Checking: %s %s' % (check.__name__, args))
        try:
            check(*args)
            ok = True
        except (OSError, subprocess.CalledProcessError, ConfigError) as e:
            self.warn(str(e))
            ok = False
        self.log_file.write('\n')
        return ok

    def find_working(self, desc, arg_name, defaults, require=()):
        self.desc_map[arg_name] = desc

        for key in require:
            if getattr(self.i, key) is None:
                self.out_skip(desc, key)
                return None

        user_choice = getattr(self.i, arg_name)
        choices = [user_choice] if user_choice else defaults
        check_one = getattr(self, 'check_' + arg_name)

        for choice in choices:
            self.out('Checking %s: %s' % (desc, choice))
            if self.do_check(check_one, choice):
                return choice

        self.out(' * Cannot find working %s; set --%s' % (desc, arg_name.replace('_', '-')),
                level='ERR')
        return None

        # Couldn't find the thing.  Print an appropriate error or warning.
        out = self.out_err if not warn_on_fail else self.out_warn

        if self.i.force:
            out('Cannot find working %s' % desc)
            if user_choice is not None:
                self.out_warn('Falling back on provided value: %s' % user_choice)
                return user_choice
            else:
                out('No fallback value is available (set --%s)' % flag)
                return None
        else:
            out('Cannot find working %s; set --%s' % (desc, flag))
            return None

    # Program checks
    def check_cc(self, cc):
        out = self.file('exe')
        src = self.write('c', 'int main() { return 37; }')
        self.run(cc, [src, '-o', out])
        self.run(out, expect_ret=37)

    def detect_cc(self):
        return self.find_working('C compiler', 'cc', ['cc', 'gcc', 'clang', 'icc'])


    def check_cxx(self, cxx):
        return self.check_cc(cxx)

    def detect_cxx(self):
        return self.find_working('C++ compiler', 'cxx', ['c++', 'g++', 'clang++', 'icpc'])


    def check_rustc(self, rustc):
        expect_version = '60926b8c5'
        output = self.run_output(rustc, ['--version'])
        if expect_version not in output:
            raise ConfigError('detected bad rust version: %r not in %r' % (expect_version, output))

    def detect_rustc(self):
        return self.find_working('Rust compiler', 'rustc', ['rustc'])


    def check_python3(self, python3):
        expect_version = (3, 4)
        output = self.run_output(python3,
                ['-c', 'import sys; print(sys.version_info >= %r)' % (expect_version,)])
        if output.strip() != 'True':
            raise ConfigError('detected bad python version: not >= %r' % (expect_version,))

    def detect_python3(self):
        return self.find_working('Python 3 interpreter', 'python3',
                [sys.executable, 'python3', 'python'])


    def check_python3_config(self, python3_config):
        self.run(python3_config, ['--help'])

    def detect_python3_config(self):
        return self.find_working('Python 3 configuration helper',
                'python3_config', [(self.i.python3 or '') + '-config', 'python3-config'],
                require=('python3',))


    def check_emscripten_fastcomp_prefix(self, prefix):
        if prefix == '':
            llc = 'llc'
        else:
            llc = os.path.join(prefix, 'bin', 'llc')

        self.run(llc, ['-march=js'])

    def detect_emscripten_fastcomp_prefix(self):
        return self.find_working(
                'emscripten-fastcomp installation',
                'emscripten_fastcomp_prefix',
                ['', '/usr', '/usr/local'])


    def check_emscripten_passes_prefix(self, prefix):
        opt = os.path.join(self.i.emscripten_fastcomp_prefix, 'bin', 'opt')

        def check(shlib, flag):
            shlib_path = os.path.join(prefix, shlib)
            # `opt` version 3.4 returns 1 on -help/-version for some reason.
            output = self.run_output(opt, ['-load', shlib_path, '-help'], expect_ret=None)
            if flag not in output:
                raise ConfigError('failed to load plugin %s' % shlib)

        check('RemoveOverflowChecks.so', '-remove-overflow-checks')
        check('RemoveAssume.so', '-remove-assume')

    def detect_emscripten_passes_prefix(self):
        return self.find_working(
                'rust-emscripten-passes build directory',
                'emscripten_passes_prefix',
                [''],
                require=('emscripten_fastcomp_prefix',))


    def check_closure_compiler(self, prog):
        # For some reason `closure-compiler --help` returns 255.  I'm not sure how consistent this
        # is, so just ignore the return code.  (We'll still get an exception if the program is not
        # found.)
        self.run(prog, ['--help'], expect_ret=None)

    def detect_closure_compiler(self):
        return self.find_working('Closure Compiler', 'closure_compiler', ['closure-compiler'])


    def check_yui_compressor(self, prog):
        self.run(prog, ['--help'])

    def detect_yui_compressor(self):
        return self.find_working('YUI Compressor', 'yui_compressor', ['yui-compressor'])


    # Feature/library checks
    def check_rust_library(self, lib, use_extern):
        src_file = self.write('rs', 'extern crate %s; fn main() {}' % lib)
        args = [src_file, '-o', '%s.exe' % src_file] + shlex.split(self.i.rust_lib_externs)
        if self.i.rust_extra_libdir is not None:
            args += ['-L', self.i.rust_extra_libdir]
            if use_extern:
                path = os.path.join(self.i.rust_extra_libdir, 'lib%s.rlib' % lib)
                args += ['--extern', '%s=%s' % (lib, path)]
        self.run(self.i.rustc, args)

    def detect_rust_libraries(self, libs):
        if self.i.rustc is None:
            self.out_skip('Rust libraries', 'rustc')
            return set()

        found_libs = set()
        libdir = self.i.rust_extra_libdir
        for lib in libs:
            self.out('Checking Rust libraries: %s' % lib)

            if self.do_check(self.check_rust_library, lib, False):
                found_libs.add(lib)
                continue

            if libdir is not None and self.do_check(self.check_rust_library, lib, True):
                found_libs.add(lib)
                self.log('Adding --extern for library %s' % lib)
                self.i.rust_lib_externs += ' --extern %s=%s' % \
                        (lib, os.path.join(libdir, 'lib%s.rlib' % lib))
                continue

            self.out(' * Cannot find working %r library' % lib, level='ERR')

        self.i.found_rust_libs = found_libs
        return set(libs) - found_libs

    def check_python_library(self, lib):
        self.run(self.i.python3, ['-c', 'import %s' % lib])

    def detect_python_libraries(self, libs):
        if self.i.python3 is None:
            self.out_skip('Python libraries', 'python3')
            return set()

        found_libs = set()
        for lib in libs:
            self.out('Checking Python libraries: %s' % lib)

            if self.do_check(self.check_python_library, lib):
                found_libs.add(lib)
                continue

            self.out(' * Cannot find working %r library' % lib, level='ERR')

        self.i.found_python3_libs = found_libs
        return set(libs) - found_libs


    # Main entry point

    def configure(self):
        errs = []


        if self.i.data_only and self.i.prebuilt_dir is None:
            errs.append('--prebuilt-dir is required if --data-only is set')


        def detect(key):
            f = getattr(self, 'detect_' + key)
            val = f()
            if val is None:
                errs.append('Cannot find working %s' % self.desc_map[key])
            setattr(self.i, key, val)

        def detect_all(*keys):
            for k in keys:
                detect(k)

        detect('python3')

        if not self.i.data_only:
            detect_all('cc', 'cxx', 'rustc', 'python3_config',
                    'emscripten_fastcomp_prefix', 'emscripten_passes_prefix',
                    'closure_compiler', 'yui_compressor')


        for missing_lib in self.detect_python_libraries(('PIL.Image', 'yaml')):
            errs.append('Cannot find Python 3 library %s' % missing_lib)

        if not self.i.data_only:
            libs = (
                    # Keep these in dependency order.  That way necessary --extern flags will already be
                    # set before they are needed for later checks.
                    'libc',
                    'bitflags',
                    'rand',
                    'regex_syntax',
                    'regex',
                    'log',
                    'env_logger',
                    'rustc_serialize',
                    'time',
                    'libsqlite3_sys',
                    'rusqlite',
                    'linked_hash_map',
                    'lru_cache',
                    )
            for missing_lib in self.detect_rust_libraries(libs):
                errs.append('Cannot find Rust library %s' % missing_lib)


        if len(errs) == 0:
            self.out('\nConfiguration OK')
        else:
            self.out('\nDetected %d error%s:' % (len(errs), 's' if len(errs) != 1 else ''))
            for e in errs:
                self.out(' * %s' % e)
        return len(errs) == 0

def run(i, log_file):
    with tempfile.TemporaryDirectory() as temp_dir:
        c = Checker(i, temp_dir, log_file)
        return c.configure()

