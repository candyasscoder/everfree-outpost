import os
import re
import sys


JS_TAG_RE = re.compile(r'''src='([a-z]*\.js)''')
REQUIRE_RE = re.compile(r'''require\(['"]([a-zA-Z0-9_/]+)['"]\)''')

def collect_deps(path):
    deps = set()
    with open(path, 'r') as f:
        for line in f:
            for match in REQUIRE_RE.finditer(line):
                deps.add(match.group(1))
    deps = sorted(deps)
    return deps

def collect_file_deps(root_path):
    root_dir = os.path.dirname(root_path)

    seen = set()
    order = []
    def walk(name):
        nonlocal root_dir, seen, order

        if name in seen:
            return
        seen.add(name)

        path = os.path.join(root_dir, '%s.js' % name)
        deps = collect_deps(path)

        for dep in deps:
            walk(dep)
        order.append(name)

    root_file = os.path.basename(root_path)
    root_name, _, _ = root_file.partition('.')
    walk(root_name)

    for name in order:
        print(name)

def main(html_path):
    base, _, ext = os.path.basename(html_path).rpartition('.')
    if base == 'client':
        base = 'main'
    collect_file_deps(os.path.join(os.path.dirname(html_path), 'js', '%s.js' % base))

if __name__ == '__main__':
    html_path, = sys.argv[1:]
    main(html_path)
