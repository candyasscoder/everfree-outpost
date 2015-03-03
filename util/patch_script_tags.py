import os
import re
import sys

def collect_deps(path):
    deps = set()
    with open(path, 'r') as f:
        for line in f:
            for match in REQUIRE_RE.finditer(line):
                deps.add(match.group(1))
    deps = sorted(deps)
    return deps

def main(root_path):
    order = ['shim.js'] + ['js/%s.js' % line.strip() for line in sys.stdin]

    base, _, ext = os.path.basename(root_path).rpartition('.')
    if base == 'client':
        base = 'outpost'

    base_js = '%s.js' % base

    with open(root_path, 'r') as f:
        for line in f:
            if base_js in line:
                for repl in order:
                    sys.stdout.write(line.replace(base_js, repl))
            else:
                sys.stdout.write(line)

if __name__ == '__main__':
    html_file, = sys.argv[1:]
    main(html_file)

