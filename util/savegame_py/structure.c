#include <Python.h>
#include <structmember.h>

#include "structure.h"
#include "entity.h"
#include "inventory.h"

#include "common.h"
#include "extra.h"
#include "reader.h"

typedef struct {
    uint32_t save_id;
    PyObject* extra_raw;
} StructureSave;

typedef struct _Structure {
    PyObject_HEAD

    int version;
    StructureSave* save;
    uint64_t stable_id;
    PyObject* extra;

    PyObject* offset;
    PyObject* template;
    uint32_t flags;

    PyObject* child_inventories;
} Structure;

static PyTypeObject StructureType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.Structure",
    sizeof(Structure),
};

static PyMemberDef Structure_members[] = {
    {"version", T_INT, offsetof(Structure, version), 0, NULL},
    {"stable_id", T_ULONGLONG, offsetof(Structure, stable_id), 0, NULL},
    {"extra", T_OBJECT, offsetof(Structure, extra), 0, NULL},
    {"offset", T_OBJECT, offsetof(Structure, offset), 0, NULL},
    {"template", T_OBJECT, offsetof(Structure, template), 0, NULL},
    {"flags", T_UINT, offsetof(Structure, flags), 0, NULL},
    {"child_inventories", T_OBJECT, offsetof(Structure, child_inventories), 0, NULL},
    {NULL}
};

static void Structure_dealloc(Structure* self) {
    Py_XDECREF(self->extra);
    Py_XDECREF(self->offset);
    Py_XDECREF(self->template);
    Py_XDECREF(self->child_inventories);

    if (self->save != NULL) {
        Py_XDECREF(self->save->extra_raw);
    }
    free(self->save);
}

static int Structure_init(Structure* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "", kwlist)) {
        return -1;
    }

    self->child_inventories = PyList_New(0);
    if (self->child_inventories == NULL) {
        goto fail;
    }

    return 0;

fail:
    Py_XDECREF(self->child_inventories);
    return -1;
}

PyObject* structure_get_type() {
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


Structure* structure_read(Reader* r, int version) {
    Structure* s = (Structure*)PyObject_CallObject((PyObject*)&StructureType, NULL);
    FAIL_IF(s == NULL);

    s->version = version;
    s->save = calloc(sizeof(StructureSave), 1);

    READ(s->save->save_id);
    FAIL_IF(read_register_object(r, s->save->save_id, (PyObject*)s) < 0);
    READ(s->stable_id);


    s->offset = (PyObject*)v3_read(r);
    FAIL_IF(s->offset == NULL);

    s->template = read_decode_template_name(r);
    FAIL_IF(s->template == NULL);

    if (version >= 4) {
        READ(s->flags);
    }


    s->save->extra_raw = extra_read(r, version);
    FAIL_IF(s->save->extra_raw  == NULL);


    uint32_t count;
    READ(count);
    for (uint32_t i = 0; i < count; ++i) {
        Inventory* obj = inventory_read(r, version);
        FAIL_IF(obj == NULL);
        FAIL_IF(PyList_Append(s->child_inventories, (PyObject*)obj) == -1);
    }

    return s;

fail:
    Py_XDECREF(s);
    return NULL;
}

int structure_read_post(Reader* r, Structure* s, int version) {
    s->extra = extra_read_post(r, s->save->extra_raw, version);
    FAIL_IF(s->extra == NULL);

    Py_DECREF(s->save->extra_raw);
    free(s->save);
    s->save = NULL;


    Py_ssize_t len;
    len = PyList_Size(s->child_inventories);
    for (Py_ssize_t i = 0; i < len; ++i) {
        PyObject* item = PyList_GetItem(s->child_inventories, i);
        FAIL_IF(item == NULL);
        FAIL_IF(inventory_read_post(r, (Inventory*)item, version) < 0);
    }

    return 0;

fail:
    return -1;
}
