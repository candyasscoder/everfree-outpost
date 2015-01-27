from fnmatch import fnmatch
import os
from pprint import pprint
import re
import sys

import yaml


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
    mk(div('title', info['title']))
    mk(div('author', 'By ' + info['author']))
    mk(div('license', marker('License') + info['license']))

    if 'url' in info:
        mk(div('download', marker('Download') + link(info['url'], 'link')))

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
                info['derived-from'] = df
                for x in df:
                    go(x)

    for x in filenames:
        go(x)

    if len(errors) > 0:
        raise ValueError('errors collecting entries:\n' + '\n'.join(errors))

    return result

def main():
    ss = Sources()

    filenames = [s.strip() for s in sys.stdin.readlines()]

    dct = collect_entries(ss, filenames)
    entries = merge_entries(dct)

    print('''
        <html>
            <head>
                <title>Everfree Outpost - Credits</title>
            </head>
            <body>
                <h1 class='top-title'>Everfree Outpost</h1>
        ''')
    for e in entries:
        print('<hr>')
        print(render_one(e, dct))
    print('''
            </body>
        </html>
        ''')

if __name__ == '__main__':
    main()
