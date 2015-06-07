#ifndef OUTPOST_SAVEGAME_ENTITY_H
#define OUTPOST_SAVEGAME_ENTITY_H

#include <Python.h>

#include "reader.h"

typedef struct _Entity Entity;

PyObject* entity_get_type();
Entity* entity_read(Reader* r, int version);
int entity_read_post(Reader* r, Entity* e, int version);

PyObject* motion_get_type();

#endif // OUTPOST_SAVEGAME_ENTITY_H
