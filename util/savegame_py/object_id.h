#ifndef OUTPOST_SAVEGAME_OBJECT_ID_H
#define OUTPOST_SAVEGAME_OBJECT_ID_H

#include <Python.h>
#include "common.h"
#include "reader.h"

typedef struct {
    PyObject_HEAD
    uint32_t id;
} AnyId;

typedef struct {
    PyObject_HEAD
    uint64_t id;
} AnyStableId;


#define FOR_EACH_OBJECT_TYPE(m) \
    m(Client, client) \
    m(Entity, entity) \
    m(Inventory, inventory) \
    m(Plane, plane) \
    m(TerrainChunk, terrain_chunk) \
    m(Structure, structure)


#define GEN_OBJECT_ID_PROTOTYPES(Obj, obj) \
    PyObject* obj##_id_get_type(); \
    PyObject* stable_##obj##_id_get_type(); \
    extern PyTypeObject Obj##IdType; \
    extern PyTypeObject Stable##Obj##IdType;

FOR_EACH_OBJECT_TYPE(GEN_OBJECT_ID_PROTOTYPES)

#undef GEN_OBJECT_ID_PROTOTYPES

PyObject* world_get_type();
extern PyTypeObject WorldType;


static inline PyObject* object_id_read(Reader* r, PyTypeObject* ty) {
    uint32_t id;
    READ(id);
    return PyObject_CallFunction((PyObject*)ty, "I", id);

fail:
    return NULL;
}

static inline PyObject* stable_id_read(Reader* r, PyTypeObject* ty) {
    uint64_t id;
    READ(id);
    return PyObject_CallFunction((PyObject*)ty, "K", id);

fail:
    return NULL;
}


#endif // OUTPOST_SAVEGAME_OBJECT_ID_H
