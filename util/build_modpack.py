import os
import platform
import subprocess
import sys


def check_py3(exe):
    try:
        ret = subprocess.call([exe, '-c', 'import sys; assert sys.version_info >= (3, 4)'])
        return ret == 0
    except OSError:
        return False

def detect_py3():
    if platform.system() != 'Windows':
        candidates = [sys.executable, 'python3', 'python']
    else:
        # Try executables from %PATH%, plus the default install locations
        candidates = [sys.executable, 'python3.exe', 'python.exe']
        candidates.extend(sorted(glob.glob('c:\\python*\\python.exe'), reverse=True))

    for exe in candidates:
        if check_py3(exe):
            return exe
    return None

def main(mods):
    py3 = detect_py3()
    if py3 is None:
        print('Could not find Python 3.4 or greater.')
        print('See README.txt for more information.')
        sys.exit(1)


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
            '--python3', py3,
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

if __name__ == '__main__':
    mods, = sys.argv[1:]
    main(mods)
