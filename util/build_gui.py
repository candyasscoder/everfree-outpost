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

    def start(self, extra_args=()):
        if self.process is not None:
            self.extra_pending += 1
            self.stop()

        self.process = subprocess.Popen(tuple(self.cmd) + tuple(extra_args),
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

        self.build = ProcessMonitor((sys.executable, 'util/build_modpack.py'))
        self.build.on_event = self._handle_build_event

        self.master.protocol('WM_DELETE_WINDOW', self._handle_close)
        self.after(100, self._check_queues)

    def _init_ui(self):
        wrapper = ttk.Frame(self)
        frame_mods = self._init_mod_frame(wrapper)
        frame_output = self._init_output_frame(wrapper)

        frame_mods.pack(pady=3)
        frame_output.pack(fill='x', pady=3)
        wrapper.pack()

    def _init_mod_frame(self, master):
        frame = ttk.LabelFrame(master, text='Mod Configuration')

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

    def _init_output_frame(self, master):
        frame = ttk.LabelFrame(master, text='Build Output')
        ttk.Button(frame, text='Start Build', command=self._start_build).pack()
        self.text_output = ScrolledText(frame, height=10, width=60)
        self.text_output.pack(fill='x')
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

    def _start_build(self):
        self.text_output.delete('1.0', tk.END)
        append_text(self.text_output, 'Building...\n')
        self.build.start((','.join(self.mods_enabled),))

    def _check_queues(self):
        self.build.poll()
        self.after(100, self._check_queues)

    def _handle_build_event(self, kind, data):
        if kind == 'output':
            append_text(self.text_output, data.decode())
        elif kind == 'result':
            if data == 0:
                append_text(self.text_output,
                        '\n\nBuild finished in %s' % (os.path.join(os.getcwd(), 'dist') + os.sep))
        elif kind == 'error':
            append_text(self.text_output, '\n\nError collecting build output: %s' % data)


    def _handle_close(self):
        self.quit()


def main():
    app = Application()
    app.mainloop()

if __name__ == '__main__':
    main()

