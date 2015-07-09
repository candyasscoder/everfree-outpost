import sys
py3 = sys.version_info >= (3,)

import os
import socket
import subprocess
import sys
import threading

if py3:
    import queue
    import tkinter as tk
    import tkinter.ttk as ttk
    from tkinter.scrolledtext import ScrolledText
else:
    import Queue as queue
    import Tkinter as tk
    import ttk
    from ScrolledText import ScrolledText

import platform
win32 = platform.system() == 'Windows'


class ProcessMonitor(threading.Thread):
    def __init__(self, process, queue):
        super(ProcessMonitor, self).__init__()
        self.daemon = True

        self.queue = queue
        self.process = process

    def run(self):
        while True:
            try:
                buf = self.process.stdout.raw.read(4096)
            except IOError as e:
                self.queue.put(('error', str(e)))
                return

            if len(buf) == 0:
                # The process closed stdout/stderr.
                self.process.wait()
                self.queue.put(('result', self.process.returncode))
                return

            self.queue.put(('output', buf))

def dequeue_all(q):
    while True:
        try:
            yield q.get(block=False)
        except queue.Empty:
            return

def append_text(widget, text):
    widget.insert(tk.END, text)
    widget.see(tk.END)

def send_control(msg):
    if win32:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.connect(('localhost', 8890))
    else:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.connect('./control')
    s.send(msg.encode() + b'\n')
    s.close()

class Application(ttk.Frame):
    def __init__(self, master=None):
        # ttk.Frame is an old-style class on 2.7
        ttk.Frame.__init__(self, master)
        self._init_ui()
        self.pack()

        self.wrapper_process = None
        self.wrapper_queue = queue.Queue()
        self.http_process = None
        self.http_queue = queue.Queue()
        self.stopping = False

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
        #notebook.add(self._init_repl_frame(notebook), text='REPL')

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
                '-- Press Ctrl-Enter to run command\n')
        self.repl_input.pack()
        self.repl_output = ScrolledText(frame, height=5, width=80)
        self.repl_output.pack()

        return frame

    def _check_one_queue(self, queue, log, on_exit):
        for kind, data in dequeue_all(queue):
            if kind == 'error':
                append_text(log, '\n\nError reading output: %s' % data)
            elif kind == 'result':
                append_text(log, '\n\nProcess exited with code %d' % data)
                on_exit(data)
            elif kind == 'output':
                append_text(log, data.decode())

    def _check_queues(self):
        def wrapper_exit(code):
            if self.stopping > 0:
                self._update_status(wrapper='down')
                self.stopping -= 1
            else:
                self._update_status(wrapper='err')

        def http_exit(code):
            if self.stopping > 0:
                self._update_status(http='down')
                self.stopping -= 1
            else:
                self._update_status(http='err')

        self._check_one_queue(self.wrapper_queue, self.wrapper_log, wrapper_exit)
        self._check_one_queue(self.http_queue, self.http_log, http_exit)

        self.after(100, self._check_queues)

    def _start(self):
        if os.name == 'posix':
            if os.path.exists('control'):
                os.remove('control')
            if os.path.exists('repl'):
                os.remove('repl')

        env = os.environ.copy()
        env['RUST_LOG'] = 'info'
        try:
            self.wrapper_process = subprocess.Popen(('bin/wrapper',),
                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                    env=env)
            ProcessMonitor(self.wrapper_process, self.wrapper_queue).start()
            wrapper_ok = 'up'
        except OSError as e:
            append_text(self.wrapper_log, '\n\nError starting main server: %s' % e)
            wrapper_ok = 'err'

        try:
            module = 'http.server' if py3 else 'SimpleHTTPServer'
            self.http_process = subprocess.Popen(
                    (sys.executable, '-u', '-m', module, '8892'),
                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                    cwd=os.path.join(os.getcwd(), 'www'))
            ProcessMonitor(self.http_process, self.http_queue).start()
            http_ok = 'up'
        except OSError as e:
            append_text(self.http_log, '\n\nError starting HTTP server: %s' % e)
            http_ok = 'err'

        self._update_status(wrapper=wrapper_ok, http=http_ok)
        self.btn_start.state(['disabled'])
        self.btn_stop.state(['!disabled'])

    def _stop(self):
        self.stopping = 2

        if self.wrapper_process is not None:
            try:
                send_control('shutdown')
            except OSError as e:
                append_text(self.wrapper_log, '\n\nError sending shutdown command: %s' % e)
                self.wrapper_process.terminate()

        if self.http_process is not None:
            self.http_process.terminate()

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

