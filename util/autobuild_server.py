from http.server import SimpleHTTPRequestHandler
import os
from socketserver import TCPServer
from time import sleep
import subprocess
import sys

DIST_DIR = os.path.realpath('dist')

class MyHandler(SimpleHTTPRequestHandler):
    def do_GET(self, *args, **kwargs):
        print('')

        rel_path = self.path[1:]
        assert os.pardir not in rel_path
        assert not rel_path.startswith('-')

        target = os.path.join('dist', rel_path)
        subprocess.call(['make', target])

        self.path = os.path.join('/dist', rel_path)

        return super(MyHandler, self).do_GET(*args, **kwargs)

TCPServer(('', int(sys.argv[1])), MyHandler).serve_forever()
