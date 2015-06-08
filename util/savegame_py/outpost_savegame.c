#include <Python.h>

#include "common.h"
#include "object_id.h"
#include "reader.h"

#include "client.h"
#include "entity.h"
#include "inventory.h"
#include "plane.h"
#include "terrain_chunk.h"
#include "structure.h"


static PyObject* py_load_client(PyObject* self, PyObject* args) {
    PyObject* bytes = NULL;
    Client* result = NULL;

    FAIL_IF(!PyArg_ParseTuple(args, "O", &bytes));

    Reader rs = {0};
    FAIL_IF(reader_init(&rs, bytes) < 0);
    Reader* r = &rs;

    uint32_t version;
    READ(version);
    result = client_read(r, version);
    FAIL_IF(result == NULL);
    FAIL_IF(client_read_post(r, result, version));

    return (PyObject*)result;

fail:
    SET_EXC();
    Py_XDECREF(result);
    return NULL;
}


static PyObject* py_load_plane(PyObject* self, PyObject* args) {
    PyObject* bytes = NULL;
    Plane* result;

    FAIL_IF(!PyArg_ParseTuple(args, "O", &bytes));

    Reader rs = {0};
    FAIL_IF(reader_init(&rs, bytes) < 0);
    Reader* r = &rs;

    uint32_t version;
    READ(version);
    result = plane_read(r, version);
    FAIL_IF(result == NULL);
    FAIL_IF(plane_read_post(r, result, version));

    return (PyObject*)result;

fail:
    SET_EXC();
    Py_XDECREF(result);
    return NULL;
}


static PyObject* py_load_terrain_chunk(PyObject* self, PyObject* args) {
    PyObject* bytes = NULL;
    TerrainChunk* result;

    FAIL_IF(!PyArg_ParseTuple(args, "O", &bytes));

    Reader rs = {0};
    FAIL_IF(reader_init(&rs, bytes) < 0);
    Reader* r = &rs;

    uint32_t version;
    READ(version);
    result = terrain_chunk_read(r, version);
    FAIL_IF(result == NULL);
    FAIL_IF(terrain_chunk_read_post(r, result, version));

    return (PyObject*)result;

fail:
    SET_EXC();
    Py_XDECREF(result);
    return NULL;
}


static struct PyMethodDef methods[] = {
    {"load_client", py_load_client, METH_VARARGS, NULL},
    {"load_plane", py_load_plane, METH_VARARGS, NULL},
    {"load_terrain_chunk", py_load_terrain_chunk, METH_VARARGS, NULL},
    {NULL, NULL, 0, NULL}
};

static struct PyModuleDef module = {
    PyModuleDef_HEAD_INIT,
    "outpost_savegame",
    NULL,
    -1,
    methods,
};

PyMODINIT_FUNC PyInit_outpost_savegame() {
    PyObject* m;
    m = PyModule_Create(&module);
    if (m == NULL)
        return NULL;

#define ADD(name, typ) \
    do { \
        PyObject* t = (typ); \
        Py_INCREF(t); \
        PyModule_AddObject(m, name, typ); \
    } while(0)

    ADD("Client", client_get_type());
    ADD("Entity", entity_get_type());
    ADD("Inventory", inventory_get_type());
    ADD("Plane", plane_get_type());
    ADD("TerrainChunk", terrain_chunk_get_type());
    ADD("Structure", structure_get_type());

    ADD("Motion", motion_get_type());

#define ADD_OBJECT_IDS(Obj, obj) \
    ADD(#Obj "Id", obj##_id_get_type()); \
    ADD("Stable" #Obj "Id", stable_##obj##_id_get_type());

    FOR_EACH_OBJECT_TYPE(ADD_OBJECT_IDS);

    ADD("World", world_get_type());
    ADD("V3", v3_get_type());
    ADD("V2", v2_get_type());

    return m;
}
