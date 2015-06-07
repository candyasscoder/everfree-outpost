#ifndef OUTPOST_SAVEGAME_STRUCTURE_H
#define OUTPOST_SAVEGAME_STRUCTURE_H

#include <Python.h>

#include "reader.h"

typedef struct _Structure Structure;

PyObject* structure_get_type();
Structure* structure_read(Reader* r, int version);
int structure_read_post(Reader* r, Structure* c, int version);

#endif // OUTPOST_SAVEGAME_STRUCTURE_H
