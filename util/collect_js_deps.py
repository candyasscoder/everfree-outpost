import os
import re
import sys


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

    return order

def main(js_src, out_file, dep_file):
    modules = collect_file_deps(js_src)

    base_dir = os.path.dirname(js_src)
    deps = [os.path.join(base_dir, '%s.js' % m) for m in modules]

    with open(dep_file, 'w') as f:
        f.write('%s:\\\n' % out_file)
        for d in deps:
            f.write('    %s\\\n' % d)

    for d in deps:
        print(d)

if __name__ == '__main__':
    js_src, out_file, dep_file = sys.argv[1:]
    main(js_src, out_file, dep_file)
