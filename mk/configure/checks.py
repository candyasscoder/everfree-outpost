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

    def out(self, msg):
        self.log(msg)
        print(msg)

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
        for key in require:
            if getattr(self.i, key) is None:
                self.out(' * Skipping check for %s because %s was not found' %
                        (desc, self.desc_map[key]))
                return None

        self.desc_map[arg_name] = desc

        user_choice = getattr(self.i, arg_name)
        choices = [user_choice] if user_choice else defaults
        check_one = getattr(self, 'check_' + arg_name)

        for choice in choices:
            self.out('Checking %s: %s' % (desc, choice))
            if self.do_check(check_one, choice):
                return choice

        self.out(' * Cannot find working %s; set --%s' % (desc, arg_name.replace('_', '-')))
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

    # Checks
    def check_cc(self, cc):
        out = self.file('exe')
        src = self.write('c', 'int main() { return 37; }')
        self.run(cc, [src, '-o', out])
        self.run(out, expect_ret=37)

    def check_cxx(self, cxx):
        return self.check_cc(cxx)

    def check_rustc(self, rustc):
        expect_version = '60926b8c5'
        output = self.run_output(rustc, ['--version'])
        if expect_version not in output:
            raise ConfigError('detected bad rust version: %r not in %r' % (expect_version, output))

    def check_python3(self, python3):
        expect_version = (3, 4)
        output = self.run_output(python3,
                ['-c', 'import sys; print(sys.version_info >= %r)' % (expect_version,)])
        if output.strip() != 'True':
            raise ConfigError('detected bad python version: not >= %r' % (expect_version,))

    def check_python3_config(self, python3_config):
        self.run(python3_config, ['--help'])

    def check_emscripten_fastcomp_prefix(self, prefix):
        if prefix == '':
            llc = 'llc'
        else:
            llc = os.path.join(prefix, 'bin', 'llc')

        self.run(llc, ['-march=js'])

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

    def check_closure_compiler(self, prog):
        # For some reason `closure-compiler --help` returns 255.  I'm not sure how consistent this
        # is, so just ignore the return code.  (We'll still get an exception if the program is not
        # found.)
        self.run(prog, ['--help'], expect_ret=None)

    def check_yui_compressor(self, prog):
        self.run(prog, ['--help'])

    def do_all(self):
        self.i.cc = self.find_working('C compiler', 'cc', ['cc', 'gcc', 'clang', 'icc'])
        self.i.cxx = self.find_working('C++ compiler', 'cxx', ['c++', 'g++', 'clang++', 'icpc'])

        self.i.rustc = self.find_working('Rust compiler', 'rustc', ['rustc'])

        self.i.python3 = self.find_working('Python 3 interpreter', 'python3',
                [sys.executable, 'python3', 'python'])
        self.i.python3_config = self.find_working('Python 3 configuration helper',
                'python3_config', [self.i.python3 + '-config', 'python3-config'],
                require=('python3',))

        self.i.emscripten_fastcomp_prefix = self.find_working(
                'emscripten-fastcomp installation',
                'emscripten_fastcomp_prefix',
                ['', '/usr', '/usr/local'])
        self.i.emscripten_passes_prefix = self.find_working(
                'rust-emscripten-passes build directory',
                'emscripten_passes_prefix',
                [''],
                require=('emscripten_fastcomp_prefix',))

        self.i.closure_compiler = self.find_working(
                'Closure Compiler',
                'closure_compiler',
                ['closure-compiler'])

        self.i.yui_compressor = self.find_working(
                'YUI Compressor',
                'yui_compressor',
                ['yui-compressor'])



def run(i):
    with tempfile.TemporaryDirectory() as temp_dir, \
            open('config.log', 'w') as log_file:
        c = Checker(i, temp_dir, log_file)
        c.do_all()
