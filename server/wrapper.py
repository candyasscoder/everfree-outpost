import socket
import struct

import tornado.autoreload
import tornado.gen
import tornado.ioloop
import tornado.iostream
import tornado.process
import tornado.web
import tornado.websocket


OP_ADD_CLIENT = 0xff00
OP_REMOVE_CLIENT = 0xff01
OP_CLIENT_REMOVED = 0xff02


class WSHandler(tornado.websocket.WebSocketHandler):
    def open(self):
        self.id = backend.add_client(self)

    def on_message(self, message):
        backend.send_client_message(self.id, message)

    def on_close(self):
        backend.remove_client(self.id)

class FileHandler(tornado.web.StaticFileHandler):
    @classmethod
    def get_absolute_path(cls, root, path):
        if path == '':
            return super(FileHandler, cls).get_absolute_path(root, 'client.html')
        return super(FileHandler, cls).get_absolute_path(root, path)

application = tornado.web.Application([
    (r'/ws', WSHandler),
    (r'/(.*)', FileHandler, { 'path': './dist/www' }),
], debug=True)


class BackendStream(object):
    def __init__(self, args, io_loop=None):
        self.io_loop = io_loop or tornado.ioloop.IOLoop.current()
        self.impl = tornado.process.Subprocess(
                args,
                stdout=tornado.process.Subprocess.STREAM,
                stdin=tornado.process.Subprocess.STREAM,
                io_loop=self.io_loop)

        self.clients = {}
        self.unused_ids = set()
        self.closed_ids = set()

        self.io_loop.add_future(self.do_read(), lambda f: print('read done?', f.result()))
        self.impl.set_exit_callback(self.on_close)

    def add_client(self, conn):
        if len(self.unused_ids) > 0:
            id = self.unused_ids.pop()
        else:
            id = len(self.clients)

        assert id < 1 << 16

        self.clients[id] = conn
        self.send_special_client_message(id, OP_ADD_CLIENT)
        return id

    def remove_client(self, id):
        self.closed_ids.add(id)
        self.send_special_client_message(id, OP_REMOVE_CLIENT)

    def send_client_message(self, id, msg):
        if len(msg) < 2 or len(msg) >= 1 << 16:
            self.remove_client(id)
            return

        if isinstance(msg, str):
            msg = msg.encode()

        header = struct.pack('HH', id, len(msg))
        self.impl.stdin.write(header)
        self.impl.stdin.write(msg)

    def send_special_client_message(self, id, opcode):
        self.impl.stdin.write(struct.pack('HHH', id, 2, opcode))

    @tornado.gen.coroutine
    def do_read(self):
        while True:
            header = yield tornado.gen.Task(self.impl.stdout.read_bytes, 4)
            id, size = struct.unpack('HH', header)
            body = yield tornado.gen.Task(self.impl.stdout.read_bytes, size)

            if size == 2:
                # It might be a control message.
                opcode, = struct.unpack('H', body)

                if opcode == OP_CLIENT_REMOVED:
                    if id in self.closed_ids:
                        self.closed_ids.remove(id)
                        del self.clients[id]
                        self.unused_ids.add(id)
                    continue

            # Only dispatch the message if the target is still connected.
            if id not in self.closed_ids:
                self.clients[id].write_message(body, binary=True)

    def on_close(self, code):
        print('closed!')

if __name__ == "__main__":
    exe_path = './dist/bin/backend'
    blocks_json_path = './dist/data/blocks.json'
    objects_json_path = './dist/data/objects.json'

    tornado.autoreload.watch(exe_path)
    tornado.autoreload.watch(blocks_json_path)
    tornado.autoreload.watch(objects_json_path)

    backend = BackendStream([exe_path, blocks_json_path, objects_json_path])
    application.listen(8888)
    tornado.ioloop.IOLoop.instance().start()
