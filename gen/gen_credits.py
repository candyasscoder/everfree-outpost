from fnmatch import fnmatch
import os
import platform
from pprint import pprint
import re
import sys

import yaml

win32 = platform.system() == 'Windows'


def merge_entries(dct):
    unique = {}

    for k, v in dct.items():
        unique[id(v)] = v
        v.setdefault('files', []).append(k)

    return sorted(unique.values(), key=lambda v: v.get('title'))

class Sources(object):
    def __init__(self):
        self.dirs_checked = set()
        self.info = {}

    def load_for_dir(self, path):
        if path in self.dirs_checked:
            return
        self.dirs_checked.add(path)


        yaml_path = os.path.join(path, 'SOURCES.yaml')
        if os.path.isfile(yaml_path):
            with open(yaml_path, 'r') as f:
                dct = yaml.load(f)

            for k,v in dct.items():
                k = k if not win32 else k.replace('/', '\\')
                dirname, filename = os.path.split(os.path.join(path, k))
                dct = self.info.setdefault(dirname, {})
                assert filename not in dct, \
                        'multiple SOURCES.yaml files contain entries for %r' % full_k
                dct[filename] = v
                v['_base_path'] = path

        if path != os.curdir:
            self.load_for_dir(os.path.dirname(path))

    def get_entry(self, path):
        dirname, filename = os.path.split(path)
        self.load_for_dir(dirname)
        if dirname not in self.info:
            raise ValueError('no SOURCES entry for %r' % path)

        dct = self.info[dirname]
        matches = []
        for pattern in dct.keys():
            if fnmatch(filename, pattern):
                matches.append(pattern)
        
        if len(matches) == 1:
            return dct[matches[0]]
        elif len(matches) == 0:
            raise ValueError('no SOURCES entry for %r' % path)
        elif len(matches) > 1:
            raise ValueError('multiple SOURCES entries for %r: %r' % (path, matches))

def anchor(name):
    return "<a id='{name}'></a>".format(name=name)

def link(url, content):
    return "<a href='{url}'>{content}</a>".format(url=url, content=content)

def div(cls, content):
    return "<div class='{cls}'>{content}</div>".format(cls=cls, content=content)

def marker(content):
    return "<b class='marker'>{content}: </b>".format(content=content)

ENCODE_RE = re.compile(r'[^a-zA-Z0-9]+')
def encode_name(name):
    return ENCODE_RE.sub('-', name).strip('-')

def render_one(info, dct):
    divs = []
    def mk(x):
        divs.append(x)

    mk(anchor(encode_name(info['title'])))
    mk("<h2 class='title'>{title}</h2>".format(title=info['title']))
    mk(div('author', 'By ' + info['author']))
    mk(div('license', marker('License') + info['license']))

    if 'url' in info:
        mk(div('download', marker('Webpage') + link(info['url'], 'link')))

    if 'derived-from' in info:
        def go(path):
            target_info = dct[path]
            title = target_info['title']
            url = '#' + encode_name(title)
            return link(url, title)
        links = ', '.join(go(i) for i in info['derived-from'])
        mk(div('derived-from', marker('Based on') + links))

    
    files_header = div('files-header', marker('Files'))
    files_items = ['<li>%s</li>' % f for f in sorted(info['files'])]
    files_ul = '<ul>\n' + '\n'.join(files_items) + '\n</ul>'
    mk(div('files', files_header + '\n' + files_ul))

    if 'attribution' in info:
        notes = info['attribution']
        header = div('notes-header', marker('Notes'))
        if '\n ' in notes:
            notes_html = '<pre>' + notes + '</pre>'
        else:
            paras = notes.split('\n')
            notes_html = '\n'.join('<p>' + p + '</p>' for p in paras)
        body = div('notes-body', notes_html)
        mk(div('notes', header + '\n' + body))

    return '\n'.join(divs) + '\n'

def collect_entries(ss, filenames):
    result = {}
    processed = set()
    errors = []

    def go(f):
        if f in result:
            return
        try:
            info = ss.get_entry(f)
        except ValueError as e:
            errors.append(str(e))
            return

        result[f] = info

        if id(info) not in processed:
            processed.add(id(info))
            if 'derived-from' in info:
                df = info['derived-from']
                df = [os.path.join(info['_base_path'], x.strip()) for x in df.split(',')]
                df = [os.path.normpath(p) for p in df]
                info['derived-from'] = df
                for x in df:
                    go(x)

    for x in filenames:
        go(x)

    if len(errors) > 0:
        raise ValueError('errors collecting entries:\n' + '\n'.join(errors))

    return result

def main(src_dir, dest_file, inputs):
    src_dir = os.path.normpath(src_dir) + os.sep
    ss = Sources()

    content = ''
    for name in inputs:
        print(name)
        with open(name, 'r') as f:
            content += f.read()
    # Support `\` for line continuations
    content = content.replace('\\\n', ' ')

    filenames = []
    for line in content.splitlines():
        line = line.strip()
        if line.startswith('#'):
            continue
        if line == '':
            continue

        for path in line.split():
            if path.endswith(':'):
                # It's actually the `out` part of `out: in1 in2 in3`.  Ignore.
                continue

            if not os.path.isfile(path):
                continue

            path = os.path.normpath(path)
            _, ext = os.path.splitext(path)

            if ext in ('.py', '.vert', '.frag'):
                # Ignore code files.
                continue

            if path.startswith(src_dir):
                filenames.append(path)

    dct = collect_entries(ss, filenames)
    entries = merge_entries(dct)

    with open(dest_file, 'w') as f:
        f.write('''
            <html>
                <head>
                    <title>Everfree Outpost - Credits</title>
                </head>
                <body>
                    <h1 class='top-title'>Everfree Outpost &ndash; Credits</h1>
            ''')
        for e in entries:
            f.write('<hr>\n')
            f.write(render_one(e, dct))
        f.write('''
                </body>
            </html>
            ''')

if __name__ == '__main__':
    src_dir, dest_file, *inputs = sys.argv[1:]
    main(src_dir, dest_file, inputs)
