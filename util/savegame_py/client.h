#ifndef OUTPOST_SAVEGAME_CLIENT_H
#define OUTPOST_SAVEGAME_CLIENT_H

#include <Python.h>

#include "reader.h"

typedef struct _Client Client;

PyObject* client_get_type();
Client* client_read(Reader* r, int version);
int client_read_post(Reader* r, Client* c, int version);

#endif // OUTPOST_SAVEGAME_CLIENT_H
