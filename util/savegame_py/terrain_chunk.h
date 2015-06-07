#ifndef OUTPOST_SAVEGAME_TERRAIN_CHUNK_H
#define OUTPOST_SAVEGAME_TERRAIN_CHUNK_H

#include <Python.h>

#include "reader.h"

typedef struct _TerrainChunk TerrainChunk;

PyObject* terrain_chunk_get_type();
TerrainChunk* terrain_chunk_read(Reader* r, int version);
int terrain_chunk_read_post(Reader* r, TerrainChunk* c, int version);

#endif // OUTPOST_SAVEGAME_TERRAIN_CHUNK_H
