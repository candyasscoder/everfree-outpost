import sys
py3 = sys.version_info >= (3,)

import glob
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


def check_py3(exe):
    try:
        ret = subprocess.call([exe, '-c', 'import sys; assert sys.version_info >= (3, 4)'])
        return ret == 0
    except OSError:
        return False

def detect_py3():
    if not win32:
        candidates = [sys.executable, 'python3', 'python']
    else:
        # Try executables from %PATH%, plus the default install locations
        candidates = [sys.executable, 'python3.exe', 'python.exe']
        candidates.extend(sorted(glob.glob('c:\\python*\\python.exe'), reverse=True))

    for exe in candidates:
        if check_py3(exe):
            return exe
    return None


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


def append_text(widget, text):
    widget.insert(tk.END, text)
    widget.see(tk.END)

def get_selection(treeview):
    s = treeview.selection()
    if s == '':
        return None
    else:
        item, = s
        return item

def enable(button, ok):
    if ok:
        button.state(['!disabled'])
    else:
        button.state(['disabled'])

class Application(ttk.Frame):
    def __init__(self, master=None):
        # ttk.Frame is an old-style class on 2.7
        ttk.Frame.__init__(self, master)
        self._init_ui()
        self.pack()

        self.master.protocol('WM_DELETE_WINDOW', self._handle_close)
        self.after(100, self._check_queues)

    def _init_ui(self):
        frame_mods = self._init_mod_frame(self)
        frame_buttons = self._init_button_frame(self)
        frame_output = self._init_output_frame(self)

        frame_mods.pack()
        frame_buttons.pack()
        frame_output.pack()
        return

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

    def _init_mod_frame(self, master):
        frame = ttk.Frame(master)

        left = ttk.Frame(frame)
        middle = ttk.Frame(frame)
        right = ttk.Frame(frame)

        ttk.Label(left, text='Mods available:').pack()
        self.list_avail = ttk.Treeview(left, height=10, selectmode='browse')
        self.list_avail.bind('<<TreeviewSelect>>', lambda e: self._update_buttons())
        self.list_avail.pack()
        self.mods_avail = []

        ttk.Label(right, text='Mods enabled:').pack()
        self.list_enabled = ttk.Treeview(right, height=10, selectmode='browse')
        self.list_enabled.bind('<<TreeviewSelect>>', lambda e: self._update_buttons())
        self.list_enabled.pack()
        self.mods_enabled = []

        self.btn_add = ttk.Button(middle, text='=>', command=self._add_mod)
        self.btn_remove = ttk.Button(middle, text='<=', command=self._remove_mod)
        self.btn_up = ttk.Button(middle, text='Up', command=self._move_up)
        self.btn_down = ttk.Button(middle, text='Down', command=self._move_down)
        btn_rescan = ttk.Button(middle, text='Rescan', command=self._update_lists)
        for b in (self.btn_add, self.btn_remove, self.btn_up, self.btn_down, btn_rescan):
            b.pack()

        self._update_lists()

        left.pack(side='left')
        middle.pack(side='left')
        right.pack(side='right')
        return frame

    def _init_button_frame(self, master):
        frame = ttk.Frame(master)
        return frame

    def _init_output_frame(self, master):
        frame = ttk.Frame(master)
        return frame

    def _update_lists(self):
        mods = sorted(m for m in os.listdir('mods') if os.path.isdir(os.path.join('mods', m)))

        old = self.mods_avail
        new = mods
        i = 0
        j = 0
        while i < len(old) and j < len(new):
            a = old[i]
            b = new[j]
            if a == b:
                i += 1
                j += 1
            elif a < b:
                self.list_avail.delete(a)
                i += 1
            else:   # a > b
                # Everything before the current element matches in the treeview
                # and in the new mod list.  So insert at index `j`.
                self.list_avail.insert('', j, b, text=b)
                j += 1
        while i < len(old):
            self.list_avail.delete(old[i])
            i += 1
        while j < len(new):
            self.list_avail.insert('', 'end', new[j], text=new[j])
            j += 1
        self.mods_avail = mods

        new_set = set(mods)
        new_mods_enabled = []
        for m in self.mods_enabled:
            if m in new_set:
                new_mods_enabled.append(m)
            else:
                self.list_enabled.delete(m)
        self.mods_enabled = new_mods_enabled

        self._update_buttons()

    def _update_buttons(self):
        sel_avail = get_selection(self.list_avail)
        sel_enabled = get_selection(self.list_enabled)

        add_ok = sel_avail is not None and sel_avail not in self.mods_enabled
        remove_ok = sel_enabled is not None

        idx = self.list_enabled.index(sel_enabled) if sel_enabled is not None else None
        up_ok = idx is not None and idx > 0
        down_ok = idx is not None and idx < len(self.mods_enabled) - 1

        enable(self.btn_add, add_ok)
        enable(self.btn_remove, remove_ok)
        enable(self.btn_up, up_ok)
        enable(self.btn_down, down_ok)


    def _add_mod(self):
        mod = get_selection(self.list_avail)
        self.list_enabled.insert('', 'end', mod, text=mod)
        self.mods_enabled.append(mod)
        self._update_buttons()

    def _remove_mod(self):
        mod = get_selection(self.list_enabled)
        idx = self.list_enabled.index(mod)
        self.list_enabled.delete(mod)
        self.mods_enabled[idx : idx + 1] = []
        self._update_buttons()

    def _move_up(self):
        mod = get_selection(self.list_enabled)
        idx = self.list_enabled.index(mod)
        self.list_enabled.move(mod, '', idx - 1)
        self._update_buttons()

    def _move_down(self):
        mod = get_selection(self.list_enabled)
        idx = self.list_enabled.index(mod)
        self.list_enabled.move(mod, '', idx + 1)
        self._update_buttons()

    def _check_queues(self):
        pass

    def _handle_close(self):
        self.quit()


def main():
    app = Application()
    app.mainloop()

if __name__ == '__main__':
    main()

