#include <Python.h>
#include <structmember.h>
#include "ffi.h"

#define FAIL_IF(c) \
    do { \
        if (c) { \
            goto fail; \
        } \
    } while(0)

#define SET_EXC() \
    do { \
        if (PyErr_Occurred() == NULL) { \
            PyErr_SetString(PyExc_RuntimeError, __func__); \
        } \
    } while(0)


static PyObject* Chunk_create(tg_chunk* chunk);
static PyObject* Structure_create(const tg_structure* structure);


typedef struct _Worker {
    PyObject_HEAD

    tg_worker* ptr;
} Worker;

static PyTypeObject WorkerType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_terrain_gen.Worker",
    sizeof(Worker),
};

static void Worker_dealloc(Worker* self) {
    if (self->ptr) {
        worker_destroy(self->ptr);
    }
}

static int Worker_init(Worker* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {"path", NULL};
    const char* path;
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "s", kwlist, &path)) {
        return -1;
    }

    self->ptr = worker_create(path);
    return 0;
}

static PyObject* Worker_request(Worker* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {"plane_id", "x", "y", NULL};
    uint64_t pid;
    int32_t x;
    int32_t y;
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "Kii", kwlist, &pid, &x, &y)) {
        return NULL;
    }

    worker_request(self->ptr, pid, x, y);
    Py_INCREF(Py_None);
    return Py_None;
}

static PyObject* Worker_get_response(Worker* self) {
    uint64_t pid;
    int32_t x;
    int32_t y;
    tg_chunk* chunk = worker_get_response(self->ptr, &pid, &x, &y);

    PyObject* py_chunk = Chunk_create(chunk);
    if (!py_chunk) {
        chunk_free(chunk);
        return NULL;
    }

    PyObject* result = Py_BuildValue("KiiO", pid, x, y, py_chunk);
    Py_DECREF(py_chunk);
    return result;
}

static PyMethodDef Worker_methods[] = {
    {"request", (PyCFunction)Worker_request, METH_VARARGS | METH_KEYWORDS},
    {"get_response", (PyCFunction)Worker_get_response, METH_NOARGS},
    {NULL}
};

PyObject* Worker_get_type() {
    WorkerType.tp_flags = Py_TPFLAGS_DEFAULT;
    WorkerType.tp_new = PyType_GenericNew;
    WorkerType.tp_dealloc = (destructor)Worker_dealloc;
    WorkerType.tp_init = (initproc)Worker_init;
    WorkerType.tp_methods = Worker_methods;

    if (PyType_Ready(&WorkerType) < 0) {
        return NULL;
    }

    return (PyObject*)&WorkerType;
}


typedef struct _Chunk {
    PyObject_HEAD

    PyObject* blocks;
    PyObject* structures;
} Chunk;

static PyMemberDef Chunk_members[] = {
    {"blocks", T_OBJECT, offsetof(Chunk, blocks), 0, NULL},
    {"structures", T_OBJECT, offsetof(Chunk, structures), 0, NULL},
    {NULL}
};

static PyTypeObject ChunkType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_terrain_gen.Chunk",
    sizeof(Chunk),
};

static void Chunk_dealloc(Chunk* self) {
    Py_XDECREF(self->blocks);
    Py_XDECREF(self->structures);
}

static int Chunk_init(Chunk* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "", kwlist)) {
        return -1;
    }

    self->blocks = PyList_New(0);
    if (self->blocks == NULL) {
        goto fail;
    }

    self->structures = PyList_New(0);
    if (self->structures == NULL) {
        goto fail;
    }

    return 0;

fail:
    SET_EXC();
    Py_XDECREF(self->blocks);
    Py_XDECREF(self->structures);
    return -1;
}

PyObject* Chunk_get_type() {
    ChunkType.tp_flags = Py_TPFLAGS_DEFAULT;
    ChunkType.tp_new = PyType_GenericNew;
    ChunkType.tp_dealloc = (destructor)Chunk_dealloc;
    ChunkType.tp_init = (initproc)Chunk_init;
    ChunkType.tp_members = Chunk_members;

    if (PyType_Ready(&ChunkType) < 0) {
        return NULL;
    }

    return (PyObject*)&ChunkType;
}

PyObject* Chunk_create(tg_chunk* ptr) {
    Chunk* chunk = (Chunk*)PyObject_CallObject((PyObject*)&ChunkType, NULL);

    size_t len = chunk_blocks_len(ptr);
    for (size_t i = 0; i < len; ++i) {
        block_id id = chunk_get_block(ptr, i);
        PyObject* py_id = PyLong_FromLong(id);
        FAIL_IF(py_id == NULL);
        int ok = PyList_Append(chunk->blocks, py_id) != -1;
        Py_DECREF(py_id);
        FAIL_IF(!ok);
    }

    len = chunk_structures_len(ptr);
    for (size_t i = 0; i < len; ++i) {
        const tg_structure* structure = chunk_get_structure(ptr, i);
        PyObject* py_structure = Structure_create(structure);
        FAIL_IF(py_structure == NULL);
        int ok = PyList_Append(chunk->structures, py_structure) != -1;
        Py_DECREF(py_structure);
        FAIL_IF(!ok);
    }

    chunk_free(ptr);
    return (PyObject*)chunk;

fail:
    SET_EXC();
    Py_XDECREF(chunk);
    chunk_free(ptr);
    return NULL;
}


typedef struct _Structure {
    PyObject_HEAD

    int32_t x;
    int32_t y;
    int32_t z;
    template_id template;
    PyObject* extra;
} Structure;

static PyMemberDef Structure_members[] = {
    {"x", T_INT, offsetof(Structure, x), 0, NULL},
    {"y", T_INT, offsetof(Structure, y), 0, NULL},
    {"z", T_INT, offsetof(Structure, z), 0, NULL},
    {"template", T_UINT, offsetof(Structure, template), 0, NULL},
    {"extra", T_OBJECT, offsetof(Structure, extra), 0, NULL},
    {NULL}
};

static PyTypeObject StructureType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_terrain_gen.Structure",
    sizeof(Structure),
};

static void Structure_dealloc(Structure* self) {
    Py_XDECREF(self->extra);
}

static int Structure_init(Structure* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "", kwlist)) {
        return -1;
    }

    self->extra = PyDict_New();
    if (self->extra == NULL) {
        goto fail;
    }

    return 0;

fail:
    SET_EXC();
    Py_XDECREF(self->extra);
    return -1;
}

PyObject* Structure_get_type() {
    StructureType.tp_flags = Py_TPFLAGS_DEFAULT;
    StructureType.tp_new = PyType_GenericNew;
    StructureType.tp_dealloc = (destructor)Structure_dealloc;
    StructureType.tp_init = (initproc)Structure_init;
    StructureType.tp_members = Structure_members;

    if (PyType_Ready(&StructureType) < 0) {
        return NULL;
    }

    return (PyObject*)&StructureType;
}

PyObject* Structure_create(const tg_structure* ptr) {
    Structure* structure = (Structure*)PyObject_CallObject((PyObject*)&StructureType, NULL);

    structure_get_pos(ptr, &structure->x, &structure->y, &structure->z);
    structure->template = structure_get_template(ptr);

    tg_extra_iter* iter = structure_extra_iter(ptr);
    const char* key;
    size_t key_len;
    const char* value;
    size_t value_len;
    while (extra_iter_next(iter, &key, &key_len, &value, &value_len)) {
        PyObject* py_key = PyUnicode_FromStringAndSize(key, key_len);
        FAIL_IF(py_key == NULL);

        PyObject* py_value = PyUnicode_FromStringAndSize(value, value_len);
        if (py_value == NULL) {
            Py_DECREF(py_key);
            goto fail;
        }

        int ok = PyDict_SetItem(structure->extra, py_key, py_value) != -1;
        Py_DECREF(py_key);
        Py_DECREF(py_value);
        FAIL_IF(!ok);
    }

    return (PyObject*)structure;

fail:
    SET_EXC();
    Py_XDECREF(structure);
    return NULL;
}



static struct PyMethodDef methods[] = {
    {NULL, NULL, 0, NULL}
};

static struct PyModuleDef module = {
    PyModuleDef_HEAD_INIT,
    "outpost_terrain_gen",
    NULL,
    -1,
    methods,
};

PyMODINIT_FUNC PyInit_outpost_terrain_gen() {
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

    ADD("Worker", Worker_get_type());
    ADD("Chunk", Chunk_get_type());
    ADD("Structure", Structure_get_type());

    return m;
}
