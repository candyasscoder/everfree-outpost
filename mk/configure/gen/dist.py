import os

from configure.template import template
from configure.util import join, maybe


def rules(i):
    return template('''
        rule dist_stamp
            command = touch $out
            description = STAMP $out

        rule copy_file
            # Use -f to avoid "text file busy" when copying binaries
            command = cp -f $in $out
            description = COPY $out

        rule copy_dir_stamp
            command = $python3 $src/mk/misc/clone_dir.py $copy_src $copy_dest $stamp
            description = COPY $copy_dest ($stamp)
            depfile = $stamp.d
    ''', **locals())

def read_manifest(path):
    contents = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line == '' or line[0] == '#':
                continue
            dest, _, src = line.partition(': ')

            contents.append((dest, src))
    return contents

def read_filter(path):
    contents = set()
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line == '' or line[0] == '#':
                continue
            contents.add(line)
    return contents

def apply_filter(manifest, filter_):
    for i in range(len(manifest)):
        dest, src = manifest[i]
        if dest not in filter_:
            src = '$prebuilt/%s' % dest
            manifest[i] = (dest, src)

def apply_exclude(manifest, names):
    for i in range(len(manifest)):
        dest, src = manifest[i]
        if dest in names or any(dest.startswith(n + '/') for n in names):
            src = '$prebuilt/%s' % dest
            manifest[i] = (dest, src)

def from_manifest(common_path, extra_path, filter_path=None, exclude_names=None):
    contents = []

    for path in (common_path, extra_path):
        contents.extend(read_manifest(path))

    if filter_path is not None:
        apply_filter(contents, read_filter(filter_path))

    if exclude_names is not None:
        apply_exclude(contents, set(n.strip() for n in exclude_names.split(',')))


    builds = []
    def add_build(*args, **kwargs):
        builds.append(template(*args, **kwargs))

    dist_deps = []

    for dest, src in contents:
        if dest.endswith('/'):
            stamp = '$builddir/dist_%s.stamp' % dest.strip('/').replace('/', '_')
            add_build('''
                build %stamp $dist/%dest: copy_dir_stamp | %src $src/mk/misc/clone_dir.py
                    copy_src = %src
                    copy_dest = $dist/%dest
                    stamp = %stamp
            ''', **locals())
            dist_deps.append(stamp)
        else:
            add_build('''
                build $dist/%dest: copy_file %src
            ''', **locals())
            dist_deps.append('$dist/%s' % dest)

    add_build(r'''
        build $builddir/dist.stamp: dist_stamp | $
            %for d in dist_deps
            %{d} $
            %end
            %{'\n'}
    ''', **locals())

    return '\n\n'.join(builds)
