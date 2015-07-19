import glob
import os
import platform
import shutil
import subprocess
import sys

win32 = platform.system() == 'Windows'

if not win32:
    try:
        from shlex import quote
    except:
        import re
        def quote(path):
            # No shlex.quote on python2, so we use this hack.
            return re.sub(r'''['"\\ ]''', lambda m: '\\' + m.group(), path)
else:
    # cmd.exe-style quoting
    def quote(path):
        return '"%s"' % path.replace('"', '""')


def check_py3(exe):
    try:
        ret = subprocess.call([exe, '-c', 'import sys; assert sys.version_info >= (3, 4)'])
        return ret == 0
    except OSError:
        return False

def detect_py3():
    if not win32:
        candidates = [sys.executable, 'python3', 'python']
    else:
        # Try executables from %PATH%, plus the default install locations
        candidates = [sys.executable, 'python3.exe', 'python.exe']
        candidates.extend(sorted(glob.glob('c:\\python*\\python.exe'), reverse=True))

    for exe in candidates:
        if check_py3(exe):
            return exe
    return None

def check_install(py3, py_mod, pip_pkg):
    # This duplicates some logic from `configure.checks`, but it's better than
    # adding code to let ./configure install things automatically.
    ret = subprocess.call([py3, '-c', 'import %s' % py_mod])
    if ret != 0:
        print('Trying to install package %r...' % pip_pkg)
        ret = subprocess.call([py3, '-c', 'import pip; pip.main(["install", %r])' % pip_pkg])
        print('pip invocation exited with code %d' % ret)

def main(mods):
    py3 = detect_py3()
    if py3 is None:
        print('Could not find Python 3.4 or greater.')
        print('See README.txt for more information.')
        sys.exit(1)

    check_install(py3, 'yaml', 'PyYAML')
    check_install(py3, 'PIL.Image', 'pillow')


    print(' === Configuration === ')
    sys.stdout.flush()
    config_env = os.environ.copy()
    if 'PYTHONPATH' in config_env:
        config_env['PYTHONPATH'] += ':mk'
    else:
        config_env['PYTHONPATH'] = 'mk'

    config_args = [
            py3,
            '-m', 'configure',
            '--data-only',
            '--prebuilt-dir', 'prebuilt',
            '--release',
            '--python3', quote(py3),
            '--with-server-gui',
    ]
    if mods != '':
        config_args.extend(['--mods', mods])

    ret = subprocess.call(config_args, env=config_env)
    if ret != 0:
        print('Configuration failed.')
        print('See README.txt for more information.')
        sys.exit(1)
    print('\n')
    sys.stdout.flush()


    print(' === Build === ')
    sys.stdout.flush()
    ret = subprocess.call([py3, 'mk/ninja.py'])
    if ret != 0:
        print('Build failed.')
        sys.exit(1)
    print('\n')
    sys.stdout.flush()


    # TODO: SUPER HACK
    for f in glob.glob('prebuilt/bin/*.dll'):
        shutil.copy(f, 'dist/bin')

if __name__ == '__main__':
    mods, = sys.argv[1:]
    main(mods)
