#ifndef OUTPOST_SAVEGAME_INVENTORY_H
#define OUTPOST_SAVEGAME_INVENTORY_H

#include <Python.h>

#include "reader.h"

typedef struct _Inventory Inventory;

PyObject* inventory_get_type();
Inventory* inventory_read(Reader* r, int version);
int inventory_read_post(Reader* r, Inventory* i, int version);

#endif // OUTPOST_SAVEGAME_INVENTORY_H
