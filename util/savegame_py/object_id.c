#include <Python.h>
#include <structmember.h>

#include "object_id.h"

static PyMemberDef AnyId_members[] = {
    {"id", T_UINT, offsetof(AnyId, id), READONLY, NULL},
    {NULL}
};


static PyMemberDef AnyStableId_members[] = {
    {"id", T_ULONGLONG, offsetof(AnyStableId, id), READONLY, NULL},
    {NULL}
};


static int AnyId_init(AnyId* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {"id", NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "I", kwlist, &self->id)) {
        return -1;
    }
    return 0;
}

static int AnyStableId_init(AnyStableId* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {"id", NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "K", kwlist, &self->id)) {
        return -1;
    }
    return 0;
}


#define GEN_TYPE(Obj, obj) \
    PyTypeObject Obj##IdType = { \
        PyVarObject_HEAD_INIT(NULL, 0) \
        "outpost_savegame." #Obj "Id", \
        sizeof(AnyId), \
    }; \
    \
    PyObject* obj##_id_get_type() { \
        Obj##IdType.tp_flags = Py_TPFLAGS_DEFAULT; \
        Obj##IdType.tp_new = PyType_GenericNew; \
        Obj##IdType.tp_init = (initproc)AnyId_init; \
        Obj##IdType.tp_members = AnyId_members; \
        \
        if (PyType_Ready(&Obj##IdType) < 0) { \
            return NULL; \
        } \
        \
        return (PyObject*)&Obj##IdType; \
    } \
    \
    PyTypeObject Stable##Obj##IdType = { \
        PyVarObject_HEAD_INIT(NULL, 0) \
        "outpost_savegame.Stable" #Obj "Id", \
        sizeof(AnyStableId), \
    }; \
    \
    PyObject* stable_##obj##_id_get_type() { \
        Stable##Obj##IdType.tp_flags = Py_TPFLAGS_DEFAULT; \
        Stable##Obj##IdType.tp_new = PyType_GenericNew; \
        Stable##Obj##IdType.tp_init = (initproc)AnyStableId_init; \
        Stable##Obj##IdType.tp_members = AnyStableId_members; \
        \
        if (PyType_Ready(&Stable##Obj##IdType) < 0) { \
            return NULL; \
        } \
        \
        return (PyObject*)&Stable##Obj##IdType; \
    }

FOR_EACH_OBJECT_TYPE(GEN_TYPE)


typedef struct {
    PyObject_HEAD
} World;

PyTypeObject WorldType = { \
    PyVarObject_HEAD_INIT(NULL, 0) \
    "outpost_savegame.World", \
    sizeof(World), \
};

PyObject* world_get_type() {
    WorldType.tp_flags = Py_TPFLAGS_DEFAULT;
    WorldType.tp_new = PyType_GenericNew;

    if (PyType_Ready(&WorldType) < 0) {
        return NULL;
    }

    return (PyObject*)&WorldType;
}
