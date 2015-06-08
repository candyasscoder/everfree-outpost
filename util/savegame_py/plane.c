#include <Python.h>
#include <structmember.h>

#include "plane.h"

#include "common.h"
#include "extra.h"
#include "object_id.h"
#include "reader.h"

typedef struct {
    uint32_t save_id;
    PyObject* extra_raw;
} PlaneSave;

typedef struct _Plane {
    PyObject_HEAD

    int version;
    PlaneSave* save;
    uint64_t stable_id;
    PyObject* extra;

    PyObject* name;
    PyObject* saved_chunks;
} Plane;

static PyTypeObject PlaneType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.Plane",
    sizeof(Plane),
};

static PyMemberDef Plane_members[] = {
    {"version", T_INT, offsetof(Plane, version), 0, NULL},
    {"stable_id", T_ULONGLONG, offsetof(Plane, stable_id), 0, NULL},
    {"extra", T_OBJECT, offsetof(Plane, extra), 0, NULL},
    {"name", T_OBJECT, offsetof(Plane, name), 0, NULL},
    {"saved_chunks", T_OBJECT, offsetof(Plane, saved_chunks), 0, NULL},
    {NULL}
};

static void Plane_dealloc(Plane* self) {
    Py_XDECREF(self->extra);
    Py_XDECREF(self->name);
    Py_XDECREF(self->saved_chunks);

    if (self->save != NULL) {
        Py_XDECREF(self->save->extra_raw);
    }
    free(self->save);
}

static int Plane_init(Plane* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "", kwlist)) {
        return -1;
    }

    self->saved_chunks = PyDict_New();
    if (self->saved_chunks == NULL) {
        goto fail;
    }

    return 0;

fail:
    SET_EXC();
    Py_XDECREF(self->saved_chunks);
    return -1;
}

PyObject* plane_get_type() {
    PlaneType.tp_flags = Py_TPFLAGS_DEFAULT;
    PlaneType.tp_new = PyType_GenericNew;
    PlaneType.tp_dealloc = (destructor)Plane_dealloc;
    PlaneType.tp_init = (initproc)Plane_init;
    PlaneType.tp_members = Plane_members;

    if (PyType_Ready(&PlaneType) < 0) {
        return NULL;
    }

    return (PyObject*)&PlaneType;
}


Plane* plane_read(Reader* r, int version) {
    Plane* p = (Plane*)PyObject_CallObject((PyObject*)&PlaneType, NULL);
    FAIL_IF(p == NULL);

    p->version = version;
    p->save = calloc(sizeof(PlaneSave), 1);

    READ(p->save->save_id);
    FAIL_IF(read_register_object(r, p->save->save_id, (PyObject*)p) < 0);
    READ(p->stable_id);


    uint32_t name_len;
    READ(name_len);
    p->name = read_string(r, name_len);
    FAIL_IF(p->name == NULL);

    uint32_t count;
    READ(count);
    for (uint32_t i = 0; i < count; ++i) {
        PyObject* key = (PyObject*)v2_read(r);
        FAIL_IF(key == NULL);

        PyObject* value = (PyObject*)stable_id_read(r, &StableTerrainChunkIdType);
        if (value == NULL) {
            Py_DECREF(key);
            goto fail;
        }

        if (PyDict_SetItem(p->saved_chunks, key, value)) {
            Py_DECREF(key);
            Py_DECREF(value);
            goto fail;
        }

        Py_DECREF(key);
        Py_DECREF(value);
    }


    p->save->extra_raw = extra_read(r, version);
    FAIL_IF(p->save->extra_raw  == NULL);

    return p;

fail:
    SET_EXC();
    Py_XDECREF(p);
    return NULL;
}

int plane_read_post(Reader* r, Plane* p, int version) {
    p->extra = extra_read_post(r, p->save->extra_raw, version);
    FAIL_IF(p->extra == NULL);

    Py_DECREF(p->save->extra_raw);
    free(p->save);
    p->save = NULL;

    return 0;

fail:
    SET_EXC();
    return -1;
}
