#ifndef OUTPOST_SAVEGAME_EXTRA_H
#define OUTPOST_SAVEGAME_EXTRA_H

#include <Python.h>

#include "reader.h"

PyObject* extra_read(Reader* r, int version);
PyObject* extra_read_post(Reader* r, PyObject* extra, int version);

#endif // OUTPOST_SAVEGAME_EXTRA_H
