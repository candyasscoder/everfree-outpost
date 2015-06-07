#ifndef OUTPOST_SAVEGAME_PLANE_H
#define OUTPOST_SAVEGAME_PLANE_H

#include <Python.h>

#include "reader.h"

typedef struct _Plane Plane;

PyObject* plane_get_type();
Plane* plane_read(Reader* r, int version);
int plane_read_post(Reader* r, Plane* c, int version);

#endif // OUTPOST_SAVEGAME_PLANE_H
