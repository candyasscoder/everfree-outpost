#include <Python.h>
#include <structmember.h>

#include "terrain_chunk.h"
#include "structure.h"

#include "common.h"
#include "extra.h"
#include "object_id.h"
#include "reader.h"


#define CHUNK_BITS 4


typedef struct {
    uint32_t save_id;
    PyObject* extra_raw;
} TerrainChunkSave;

typedef struct _TerrainChunk {
    PyObject_HEAD

    int version;
    TerrainChunkSave* save;
    uint64_t stable_id;
    PyObject* extra;

    PyObject* blocks;

    PyObject* child_structures;
} TerrainChunk;

static PyTypeObject TerrainChunkType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.TerrainChunk",
    sizeof(TerrainChunk),
};

static PyMemberDef TerrainChunk_members[] = {
    {"version", T_INT, offsetof(TerrainChunk, version), 0, NULL},
    {"stable_id", T_ULONGLONG, offsetof(TerrainChunk, stable_id), 0, NULL},
    {"extra", T_OBJECT, offsetof(TerrainChunk, extra), 0, NULL},
    {"blocks", T_OBJECT, offsetof(TerrainChunk, blocks), 0, NULL},
    {"child_structures", T_OBJECT, offsetof(TerrainChunk, child_structures), 0, NULL},
    {NULL}
};

static void TerrainChunk_dealloc(TerrainChunk* self) {
    Py_XDECREF(self->extra);
    Py_XDECREF(self->blocks);
    Py_XDECREF(self->child_structures);

    if (self->save != NULL) {
        Py_XDECREF(self->save->extra_raw);
    }
    free(self->save);
}

static int TerrainChunk_init(TerrainChunk* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "", kwlist)) {
        return -1;
    }

    self->blocks = PyList_New(1 << (3 * CHUNK_BITS));
    if (self->blocks == NULL) {
        goto fail;
    }
    for (int i = 0; i < 1 << (3 * CHUNK_BITS); ++i) {
        Py_INCREF(Py_None);
        if (PyList_SET_ITEM(self->blocks, i, Py_None) < 0) {
            // XXX: SET_ITEM does steal the reference even on failure, right?
            goto fail;
        }
    }

    self->child_structures = PyList_New(0);
    if (self->child_structures == NULL) {
        goto fail;
    }

    return 0;

fail:
    Py_XDECREF(self->blocks);
    Py_XDECREF(self->child_structures);
    return -1;
}

PyObject* terrain_chunk_get_type() {
    TerrainChunkType.tp_flags = Py_TPFLAGS_DEFAULT;
    TerrainChunkType.tp_new = PyType_GenericNew;
    TerrainChunkType.tp_dealloc = (destructor)TerrainChunk_dealloc;
    TerrainChunkType.tp_init = (initproc)TerrainChunk_init;
    TerrainChunkType.tp_members = TerrainChunk_members;

    if (PyType_Ready(&TerrainChunkType) < 0) {
        return NULL;
    }

    return (PyObject*)&TerrainChunkType;
}


PyObject* read_block_type_table(Reader* r) {
    PyObject* block_type_table = PyDict_New();
    FAIL_IF(block_type_table == NULL);

    uint32_t block_type_count;
    READ(block_type_count);
    printf("blocks: %d\n", block_type_count);
    for (uint32_t i = 0; i < block_type_count; ++i) {
        struct {
            uint16_t old_id;
            uint8_t shape;
            uint8_t name_len;
        } x;
        READ(x);
        printf("  %d: info %d, %d, %d\n", i, x.old_id, x.shape, x.name_len);
        PyObject* name = read_string(r, x.name_len);
        FAIL_IF(name == NULL);

        PyObject* id_obj = PyLong_FromLong(x.old_id);
        if (id_obj == NULL) {
            Py_DECREF(name);
            goto fail;
        }

        if (PyDict_SetItem(block_type_table, id_obj, name) < 0) {
            Py_DECREF(name);
            Py_DECREF(id_obj);
            goto fail;
        }

        Py_DECREF(name);
        Py_DECREF(id_obj);
    }

    return block_type_table;

fail:
    Py_XDECREF(block_type_table);
    return NULL;
}

TerrainChunk* terrain_chunk_read(Reader* r, int version) {
    PyObject* block_type_table = NULL;

    TerrainChunk* tc = (TerrainChunk*)PyObject_CallObject((PyObject*)&TerrainChunkType, NULL);
    FAIL_IF(tc == NULL);

    tc->version = version;
    tc->save = calloc(sizeof(TerrainChunkSave), 1);

    READ(tc->save->save_id);
    FAIL_IF(read_register_object(r, tc->save->save_id, (PyObject*)tc) < 0);
    READ(tc->stable_id);


    uint16_t buf[1 << (3 * CHUNK_BITS)];
    READ(buf);

    block_type_table = read_block_type_table(r);
    FAIL_IF(block_type_table == NULL);
    for (int i = 0; i < 1 << (3 * CHUNK_BITS); ++i) {
        PyObject* key = PyLong_FromLong(buf[i]);
        FAIL_IF(key == NULL);

        PyObject* value = PyDict_GetItem(block_type_table, key);
        if (value == NULL) {
            Py_DECREF(key);
            goto fail;
        }

        Py_INCREF(value);
        if (PyList_SetItem(tc->blocks, i, value) < 0) {
            // XXX: SetItem does steal the reference even on failure, right?
            Py_DECREF(key);
            goto fail;
        }
    }
    Py_DECREF(block_type_table);
    block_type_table = NULL;


    // No script extras for TerrainChunk yet.
    if (version >= 999999) {
        tc->save->extra_raw = extra_read(r, version);
        FAIL_IF(tc->save->extra_raw  == NULL);
    } else {
        Py_INCREF(Py_None);
        tc->save->extra_raw = Py_None;
    }


    uint32_t count;
    READ(count);
    for (uint32_t i = 0; i < count; ++i) {
        Structure* obj = structure_read(r, version);
        FAIL_IF(obj == NULL);
        FAIL_IF(PyList_Append(tc->child_structures, (PyObject*)obj) == -1);
    }

    return tc;

fail:
    Py_XDECREF(tc);
    Py_XDECREF(block_type_table);
    return NULL;
}

int terrain_chunk_read_post(Reader* r, TerrainChunk* tc, int version) {
    tc->extra = extra_read_post(r, tc->save->extra_raw, version);
    FAIL_IF(tc->extra == NULL);

    Py_DECREF(tc->save->extra_raw);
    free(tc->save);
    tc->save = NULL;


    Py_ssize_t len;
    len = PyList_Size(tc->child_structures);
    for (Py_ssize_t i = 0; i < len; ++i) {
        PyObject* item = PyList_GetItem(tc->child_structures, i);
        FAIL_IF(item == NULL);
        FAIL_IF(structure_read_post(r, (Structure*)item, version) < 0);
    }

    return 0;

fail:
    return -1;
}
