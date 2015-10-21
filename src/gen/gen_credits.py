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
        self.thirdparty = {}

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

        yaml_path = os.path.join(path, 'THIRDPARTY.yaml')
        if os.path.isfile(yaml_path):
            with open(yaml_path, 'r') as f:
                dct = yaml.load(f)

            for k,v in dct.items():
                assert k not in self.thirdparty, \
                        'multiple THIRDPARTY.yaml files contain entries for %r' % k
                self.thirdparty[k] = v

        if path != os.curdir:
            self.load_for_dir(os.path.dirname(path))

    def get_entry(self, path):
        dirname, filename = os.path.split(path)
        self.load_for_dir(dirname)
        if dirname not in self.info:
            raise KeyError('no SOURCES entry for %r' % path)

        dct = self.info[dirname]
        matches = []
        for pattern in dct.keys():
            if fnmatch(filename, pattern):
                matches.append(pattern)
        
        if len(matches) == 1:
            return dct[matches[0]]
        elif len(matches) == 0:
            raise KeyError('no SOURCES entry for %r' % path)
        elif len(matches) > 1:
            raise KeyError('multiple SOURCES entries for %r: %r' % (path, matches))

    def get_thirdparty(self, key):
        if key not in self.thirdparty:
            raise KeyError('no THIRDPARTY entry for %r' % key)
        return self.thirdparty[key]


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


def collect_entries(ss, filenames):
    authors = set()
    thirdparty = set()
    errors = []

    for f in filenames:
        try:
            info = ss.get_entry(f)
        except (KeyError, ValueError) as e:
            errors.append(str(e))
            continue

        if 'author' in info:
            authors.update(n.strip() for n in info['author'].split(','))
        if 'derived-from' in info:
            thirdparty.update(x.strip() for x in info['derived-from'].split(','))

    if len(errors) > 0:
        raise ValueError('errors collecting entries:\n' + '\n'.join(errors))

    return authors, thirdparty

def gen_author(ss, a):
    return '%s<br>\n' % a

def gen_thirdparty(info):
    divs = []
    def mk(x):
        divs.append(x)

    mk(anchor(encode_name(info['title'])))
    mk("<h4 class='title'>{title}</h4>".format(title=info['title']))
    mk(div('author', 'By ' + info['author']))
    mk(div('license', marker('License') + info['license']))

    if 'url' in info:
        mk(div('download', marker('Website') + link(info['url'], 'link')))

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

def clean(s):
    return re.sub('[^a-zA-Z0-9 ]+', '', s).lower()

def main(src_dir, dest_file, inputs):
    src_dir = os.path.normpath(src_dir) + os.sep
    ss = Sources()

    content = ''
    for name in inputs:
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

            if ext in ('.py', '.od', '.vert', '.frag'):
                # Ignore code files.
                continue

            if path.startswith(src_dir):
                filenames.append(path)

    authors, thirdparty = collect_entries(ss, filenames)

    author_list = sorted(authors)
    thirdparty_list = [ss.get_thirdparty(t) for t in thirdparty]
    thirdparty_list.sort(key=lambda x: clean(x['title']))

    with open(dest_file, 'w') as f:
        f.write('''
            <html>
                <head>
                    <title>Everfree Outpost - Credits</title>
                </head>
                <body>
                    <h1 class='top-title'>Everfree Outpost &ndash; Credits</h1>
                    <h3>Developed by:</h3>
            ''')
        for a in author_list:
            f.write(gen_author(ss, a))
        f.write('''
            <hr>
            <h3>Includes artwork from:</h3>
            ''')
        for t in thirdparty_list:
            f.write(gen_thirdparty(t))
        f.write('''
                </body>
            </html>
            ''')

if __name__ == '__main__':
    src_dir, dest_file, *inputs = sys.argv[1:]
    main(src_dir, dest_file, inputs)
