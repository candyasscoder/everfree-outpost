#include <Python.h>
#include <structmember.h>

#include "common.h"
#include "extra.h"
#include "reader.h"

typedef struct {
    uint32_t save_id;
    PyObject* extra_raw;
} InventorySave;

typedef struct _Inventory {
    PyObject_HEAD

    int version;
    InventorySave* save;
    uint64_t stable_id;
    PyObject* extra;

    PyObject* contents;
} Inventory;

static PyTypeObject InventoryType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.Inventory",
    sizeof(Inventory),
};

static PyMemberDef Inventory_members[] = {
    {"version", T_INT, offsetof(Inventory, version), 0, NULL},
    {"stable_id", T_ULONGLONG, offsetof(Inventory, stable_id), 0, NULL},
    {"extra", T_OBJECT, offsetof(Inventory, extra), 0, NULL},
    {"contents", T_OBJECT, offsetof(Inventory, contents), 0, NULL},
    {NULL}
};

static void Inventory_dealloc(Inventory* self) {
    Py_XDECREF(self->extra);
    Py_XDECREF(self->contents);

    if (self->save != NULL) {
        Py_XDECREF(self->save->extra_raw);
    }
    free(self->save);
}

static int Inventory_init(Inventory* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "", kwlist)) {
        return -1;
    }

    self->contents = PyDict_New();
    if (self->contents == NULL) {
        goto fail;
    }

    return 0;

fail:
    SET_EXC();
    Py_XDECREF(self->contents);
    return -1;
}

PyObject* inventory_get_type() {
    InventoryType.tp_flags = Py_TPFLAGS_DEFAULT;
    InventoryType.tp_new = PyType_GenericNew;
    InventoryType.tp_dealloc = (destructor)Inventory_dealloc;
    InventoryType.tp_init = (initproc)Inventory_init;
    InventoryType.tp_members = Inventory_members;

    if (PyType_Ready(&InventoryType) < 0) {
        return NULL;
    }

    return (PyObject*)&InventoryType;
}


Inventory* inventory_read(Reader* r, int version) {
    Inventory* i = (Inventory*)PyObject_CallObject((PyObject*)&InventoryType, NULL);
    FAIL_IF(i == NULL);

    i->version = version;
    i->save = calloc(sizeof(InventorySave), 1);

    READ(i->save->save_id);
    FAIL_IF(read_register_object(r, i->save->save_id, (PyObject*)i) < 0);
    READ(i->stable_id);

    uint32_t count;

    READ(count);
    for (uint32_t j = 0; j < count; ++j) {
        struct {
            uint16_t old_id;
            uint8_t count;
            uint8_t name_len;
        } data;
        READ(data);

        PyObject* name = read_decode_item_name(r, data.old_id, data.name_len);
        FAIL_IF(name == NULL);

        PyObject* count = PyLong_FromLong(data.count);
        if (count == NULL) {
            Py_XDECREF(name);
            goto fail;
        }

        PyDict_SetItem(i->contents, name, count);
    }

    i->save->extra_raw = extra_read(r, version);
    FAIL_IF(i->save->extra_raw  == NULL);

    return i;

fail:
    SET_EXC();
    Py_XDECREF(i);
    return NULL;
}

int inventory_read_post(Reader* r, Inventory* i, int version) {
    i->extra = extra_read_post(r, i->save->extra_raw, version);
    FAIL_IF(i->extra == NULL);

    Py_DECREF(i->save->extra_raw);
    free(i->save);
    i->save = NULL;


    return 0;

fail:
    SET_EXC();
    return -1;
}
