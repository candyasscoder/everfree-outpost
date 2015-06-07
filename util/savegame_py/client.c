#include <Python.h>
#include <structmember.h>

#include "client.h"
#include "entity.h"
#include "inventory.h"

#include "common.h"
#include "extra.h"
#include "reader.h"

typedef struct {
    uint32_t save_id;
    uint32_t pawn_id;
    PyObject* extra_raw;
} ClientSave;

typedef struct _Client {
    PyObject_HEAD

    int version;
    ClientSave* save;
    uint64_t stable_id;
    PyObject* extra;

    PyObject* pawn;

    PyObject* child_entities;
    PyObject* child_inventories;
} Client;

static PyTypeObject ClientType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.Client",
    sizeof(Client),
};

static PyMemberDef Client_members[] = {
    {"version", T_INT, offsetof(Client, version), 0, NULL},
    {"stable_id", T_ULONGLONG, offsetof(Client, stable_id), 0, NULL},
    {"extra", T_OBJECT, offsetof(Client, extra), 0, NULL},
    {"pawn", T_OBJECT, offsetof(Client, pawn), 0, NULL},
    {"child_entities", T_OBJECT, offsetof(Client, child_entities), 0, NULL},
    {"child_inventories", T_OBJECT, offsetof(Client, child_inventories), 0, NULL},
    {NULL}
};

static void Client_dealloc(Client* self) {
    Py_XDECREF(self->extra);
    Py_XDECREF(self->pawn);
    Py_XDECREF(self->child_entities);
    Py_XDECREF(self->child_inventories);

    if (self->save != NULL) {
        Py_XDECREF(self->save->extra_raw);
    }
    free(self->save);
}

static int Client_init(Client* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "", kwlist)) {
        return -1;
    }

    self->child_entities = PyList_New(0);
    if (self->child_entities == NULL) {
        goto fail;
    }

    self->child_inventories = PyList_New(0);
    if (self->child_inventories == NULL) {
        goto fail;
    }

    return 0;

fail:
    Py_XDECREF(self->child_entities);
    Py_XDECREF(self->child_inventories);
    return -1;
}

PyObject* client_get_type() {
    ClientType.tp_flags = Py_TPFLAGS_DEFAULT;
    ClientType.tp_new = PyType_GenericNew;
    ClientType.tp_dealloc = (destructor)Client_dealloc;
    ClientType.tp_init = (initproc)Client_init;
    ClientType.tp_members = Client_members;

    if (PyType_Ready(&ClientType) < 0) {
        return NULL;
    }

    return (PyObject*)&ClientType;
}


Client* client_read(Reader* r, int version) {
    printf("begin reading client\n");
    Client* c = (Client*)PyObject_CallObject((PyObject*)&ClientType, NULL);
    FAIL_IF(c == NULL);

    c->version = version;
    c->save = calloc(sizeof(ClientSave), 1);

    READ(c->save->save_id);
    FAIL_IF(read_register_object(r, c->save->save_id, (PyObject*)c) < 0);
    READ(c->stable_id);

    READ(c->save->pawn_id);

    c->save->extra_raw = extra_read(r, version);
    FAIL_IF(c->save->extra_raw  == NULL);

    uint32_t count;

    READ(count);
    printf("  read %d entities for client\n", count);
    for (uint32_t i = 0; i < count; ++i) {
        Entity* obj = entity_read(r, version);
        FAIL_IF(obj == NULL);
        FAIL_IF(PyList_Append(c->child_entities, (PyObject*)obj) == -1);
    }

    READ(count);
    printf("  read %d inventories for client\n", count);
    for (uint32_t i = 0; i < count; ++i) {
        Inventory* obj = inventory_read(r, version);
        FAIL_IF(obj == NULL);
        FAIL_IF(PyList_Append(c->child_inventories, (PyObject*)obj) == -1);
    }

    return c;

fail:
    Py_XDECREF(c);
    return NULL;
}

int client_read_post(Reader* r, Client* c, int version) {
    if (c->save->pawn_id != -1) {
        c->pawn = read_find_object(r, c->save->pawn_id);
        FAIL_IF(c->pawn == NULL);
    }

    c->extra = extra_read_post(r, c->save->extra_raw, version);
    FAIL_IF(c->extra == NULL);

    Py_DECREF(c->save->extra_raw);
    free(c->save);
    c->save = NULL;


    Py_ssize_t len;

    len = PyList_Size(c->child_entities);
    for (Py_ssize_t i = 0; i < len; ++i) {
        PyObject* item = PyList_GetItem(c->child_entities, i);
        FAIL_IF(item == NULL);
        FAIL_IF(entity_read_post(r, (Entity*)item, version) < 0);
    }

    len = PyList_Size(c->child_inventories);
    for (Py_ssize_t i = 0; i < len; ++i) {
        PyObject* item = PyList_GetItem(c->child_inventories, i);
        FAIL_IF(item == NULL);
        FAIL_IF(inventory_read_post(r, (Inventory*)item, version) < 0);
    }

    return 0;

fail:
    return -1;
}
