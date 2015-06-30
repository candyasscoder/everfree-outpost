import os

from configure.template import template
from configure.util import join, maybe


def rules(i):
    return template('''
        rule dist_stamp
            command = touch $out
            description = STAMP $out

        rule copy_file
            command = cp $in $out
            description = COPY $out

        rule copy_dir_stamp
            command = $
                cp -ru $src $dest && $
                echo $out: $$(find $src) >$depfile && $
                touch $out
            description = COPY $dest ($out)
            depfile = $out.d
    ''', **locals())

def from_manifest(common_path, extra_path):
    contents = []

    for path in (common_path, extra_path):
        with open(path) as f:
            for line in f:
                line = line.strip()
                if line == '' or line[0] == '#':
                    continue
                dest, _, src = line.partition(': ')

                contents.append((dest, src))


    builds = []
    def add_build(*args, **kwargs):
        builds.append(template(*args, **kwargs))

    dist_deps = []

    for dest, src in contents:
        if dest.endswith('/'):
            stamp = '$builddir/dist_%s.stamp' % dest.strip('/').replace('/', '_')
            add_build('''
                build %stamp: copy_dir_stamp
                    src = %src
                    dest = $dist/%dest
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
