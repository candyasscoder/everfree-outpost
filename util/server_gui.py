import sys
py3 = sys.version_info >= (3,)

import os
import socket
import subprocess
import sys
import threading

if py3:
    from queue import Queue, Empty
    import tkinter as tk
    import tkinter.ttk as ttk
    from tkinter.scrolledtext import ScrolledText
else:
    from Queue import Queue, Empty
    import Tkinter as tk
    import ttk
    from ScrolledText import ScrolledText

import platform
win32 = platform.system() == 'Windows'


def dequeue_all(q):
    while True:
        try:
            yield q.get(block=False)
        except Empty:
            return


class ProcessMonitorWorker(threading.Thread):
    def __init__(self, process, queue):
        super(ProcessMonitorWorker, self).__init__()
        self.daemon = True

        self.queue = queue
        self.process = process

    def run(self):
        if hasattr(self.process.stdout, 'raw'):
            f = self.process.stdout.raw
        else:
            f = self.process.stdout

        while True:
            try:
                buf = f.read(4096)
            except IOError as e:
                self.queue.put(('error', str(e)))
                return

            if len(buf) == 0:
                # The process closed stdout/stderr.
                self.process.wait()
                self.queue.put(('result', self.process.returncode))
                return

            self.queue.put(('output', buf))

class ProcessMonitor(object):
    def __init__(self, cmd, **kwargs):
        self.cmd = cmd
        self.kwargs = kwargs
        self.process = None
        self.queue = Queue()
        self.extra_pending = 0
        self.on_event = lambda k, d: None

    def start(self):
        if self.process is not None:
            self.extra_pending += 1
            self.stop()

        self.process = subprocess.Popen(self.cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                **self.kwargs)
        ProcessMonitorWorker(self.process, self.queue).start()

    def stop(self):
        if self.process is not None:
            self.process.terminate()
            self.process = None

    def poll(self):
        for kind, data in dequeue_all(self.queue):
            if kind in ('error', 'result'):
                if self.extra_pending == 0:
                    self.process = None
                else:
                    self.extra_pending -= 1
            self.on_event(kind, data)


def send_control(msg):
    if not win32:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.connect('./control')
    else:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.connect(('localhost', 8890))
    s.send(msg.encode() + b'\n')
    s.close()

class WrapperProcessMonitor(ProcessMonitor):
    def stop(self):
        if self.process is not None:
            try:
                send_control('shutdown')
            except (IOError, OSError) as e:
                self.process.terminate()
            self.process = None


class ReplConnectionWorker(threading.Thread):
    def __init__(self, sock, queue):
        super(ReplConnectionWorker, self).__init__()
        self.daemon = True

        self.sock = sock
        self.queue = queue

    def run(self):
        while True:
            try:
                buf = self.sock.recv(4096)
            except IOError as e:
                self.queue.put(('error', str(e)))
                return

            if len(buf) == 0:
                # The process closed stdout/stderr.
                self.queue.put(('closed', None))
                return

            self.queue.put(('recv', buf))

class ReplConnection(object):
    def __init__(self):
        self.sock = None
        self.queue = Queue()
        self.on_event = lambda k, d: None

    def _ensure_open(self):
        if self.sock is not None:
            return

        if not win32:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.connect('./repl')
        else:
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.connect(('localhost', 8891))
        # Defer assignment to `self.sock` until the `conenct` has succeeded.
        self.sock = s
        ReplConnectionWorker(self.sock, self.queue).start()

    def send(self, msg):
        self._ensure_open()
        self.sock.send(msg.encode('utf-8'))

    def close(self):
        if self.sock is not None:
            self.sock.shutdown(socket.SHUT_RDWR)
            self.sock.close()
            self.sock = None

    def poll(self):
        for kind, data in dequeue_all(self.queue):
            if kind in ('error', 'closed'):
                self.close()
            self.on_event(kind, data)


def append_text(widget, text):
    widget.insert(tk.END, text)
    widget.see(tk.END)

class Application(ttk.Frame):
    def __init__(self, master=None):
        # ttk.Frame is an old-style class on 2.7
        ttk.Frame.__init__(self, master)
        self._init_ui()
        self.pack()

        env = os.environ.copy()
        env['RUST_LOG'] = 'info'
        self.wrapper = WrapperProcessMonitor(('bin/wrapper',), env=env)
        self.wrapper.on_event = self._handle_wrapper_event

        module = 'http.server' if py3 else 'SimpleHTTPServer'
        self.http = ProcessMonitor((sys.executable, '-u', '-m', module, '8892'),
            cwd=os.path.join(os.getcwd(), 'www'))
        self.http.on_event = self._handle_http_event

        self.repl = ReplConnection()
        self.repl.on_event = self._handle_repl_event

        self.stopping = 0

        self.master.protocol('WM_DELETE_WINDOW', self._handle_close)
        self.after(100, self._check_queues)

    def _init_ui(self):
        frame_top = ttk.Frame(self)
        notebook = ttk.Notebook(self)

        self.btn_start = ttk.Button(frame_top, text='Start Server', command=self._start)
        self.btn_stop = ttk.Button(frame_top, text='Stop Server', command=self._stop)
        self.btn_stop.state(['disabled'])
        self.btn_start.pack(side='left')
        self.btn_stop.pack(side='right')

        notebook.add(self._init_status_frame(notebook), text='Status')
        notebook.add(self._init_repl_frame(notebook), text='REPL')

        self.wrapper_log = ScrolledText(notebook, height=20, width=80)
        notebook.add(self.wrapper_log, text='Main Log')

        self.http_log = ScrolledText(notebook, height=20, width=80)
        notebook.add(self.http_log, text='HTTP Log')

        frame_top.pack()
        notebook.pack()

    def _init_status_frame(self, notebook):
        wrapper = ttk.Frame(notebook)
        frame = ttk.Frame(wrapper)

        ttk.Label(frame, text='Main Server: ').grid(row=0, column=0, sticky='W')
        ttk.Label(frame, text='HTTP Server: ').grid(row=1, column=0, sticky='W')

        self.status_wrapper = ttk.Label(frame, text='Not running')
        self.status_wrapper.grid(row=0, column=1, sticky='W')

        self.status_http = ttk.Label(frame, text='Not running')
        self.status_http.grid(row=1, column=1, sticky='W')

        self.status_extra = ttk.Label(frame, text='')
        self.status_extra.grid(row=2, column=0, columnspan=2)

        frame.pack()
        return wrapper

    def _init_repl_frame(self, notebook):
        frame = ttk.Frame(notebook)

        self.repl_input = ScrolledText(frame, height=15, width=80)
        self.repl_input.insert(tk.END,
                '-- REPL command input\n'
                '-- Press Ctrl-Enter to run command\n'
                'client_by_name(\'OP\'):extra().superuser = true')
        self.repl_input.bind('<Control-Return>', self._repl_send)
        self.repl_input.pack()
        self.repl_output = ScrolledText(frame, height=5, width=80)
        self.repl_output.pack()

        return frame

    def _handle_wrapper_event(self, kind, data):
        self._log_event(self.wrapper_log, kind, data)
        if kind in ('error', 'result'):
            if self.stopping > 0:
                self.stopping -= 1
                stat = 'down'
            else:
                stat = 'err'
            self._update_status(wrapper=stat)

    def _handle_http_event(self, kind, data):
        self._log_event(self.http_log, kind, data)
        if kind in ('error', 'result'):
            if self.stopping > 0:
                self.stopping -= 1
                stat = 'down'
            else:
                stat = 'err'
            self._update_status(http=stat)

    def _repl_send(self, evt):
        cmd = self.repl_input.get('1.0', tk.END)
        cmd = '{\n%s\n}\n' % cmd
        try:
            self.repl.send(cmd)
        except (IOError, OSError) as e:
            append_text(self.repl_output, '\n\nError sending command: %s' % e)
        return 'break'

    def _handle_repl_event(self, kind, data):
        if kind == 'recv':
            append_text(self.repl_output, data.decode('utf-8'))

    def _log_event(self, log, kind, data):
        if kind == 'error':
            append_text(log, '\n\nError reading output: %s' % data.decode())
        elif kind == 'result':
            append_text(log, '\n\nProcess exited with code %d' % data)
        elif kind == 'output':
            append_text(log, data.decode())

    def _check_queues(self):
        self.wrapper.poll()
        self.http.poll()
        self.repl.poll()

        self.after(100, self._check_queues)

    def _start(self):
        if os.name == 'posix':
            if os.path.exists('control'):
                os.remove('control')
            if os.path.exists('repl'):
                os.remove('repl')

        try:
            self.wrapper.start()
            wrapper_ok = 'up'
        except OSError as e:
            append_text(self.wrapper_log, '\n\nError starting main server: %s' % e)
            wrapper_ok = 'err'

        try:
            self.http.start()
            http_ok = 'up'
        except OSError as e:
            append_text(self.http_log, '\n\nError starting HTTP server: %s' % e)
            http_ok = 'err'

        self._update_status(wrapper=wrapper_ok, http=http_ok)
        self.btn_start.state(['disabled'])
        self.btn_stop.state(['!disabled'])

    def _stop(self):
        self.stopping = 2

        self.wrapper.stop()
        self.http.stop()
        self.repl.close()

        self.btn_start.state(['!disabled'])
        self.btn_stop.state(['disabled'])

    def _update_status(self, wrapper=None, http=None):
        STATUS_TEXT = {
                'up': 'Running',
                'down': 'Not running',
                'err': 'Error (see log)',
                }
        if wrapper is not None:
            self.status_wrapper.config(text=STATUS_TEXT[wrapper])

        if http is not None:
            self.status_http.config(text=STATUS_TEXT[http])

        if wrapper == 'up' and http == 'up':
            self.status_extra.config(
                    text='Visit http://localhost:8889/client.html to play')
        else:
            self.status_extra.config(text='')

    def _handle_close(self):
        self._stop()
        self.quit()


def main():
    app = Application()
    app.mainloop()

if __name__ == '__main__':
    main()

