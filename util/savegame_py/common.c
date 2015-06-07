#include <Python.h>
#include <structmember.h>

#include "common.h"
#include "reader.h"

#define SELF_ARROW(X)   self->X
#define OTHER_ARROW(X)  other->X
#define JUST_C(X)       c

#define V2_DO_OP(OP, LHS, RHS) \
    OP(LHS(x), RHS(x)), \
    OP(LHS(y), RHS(y))
#define V3_DO_OP(OP, LHS, RHS) \
    OP(LHS(x), RHS(x)), \
    OP(LHS(y), RHS(y)), \
    OP(LHS(z), RHS(z))

#define V2_REP(s)       s s
#define V3_REP(s)       s s s

#define GEN_OP(Vn, name, OP) \
    static PyObject* Vn##_##name(Vn* self, PyObject* args) { \
        PyObject* obj = NULL; \
        if (!PyArg_ParseTuple(args, "O", &obj)) { \
            return NULL; \
        } \
        \
        PyObject* result = NULL; \
        \
        if (PyLong_Check(obj)) { \
            int overflow = 0; \
            int32_t c = PyLong_AsLongAndOverflow(obj, &overflow); \
            if (!overflow) { \
                result = PyObject_CallFunction((PyObject*)&Vn##Type, \
                        Vn##_REP("i"), \
                        Vn##_DO_OP(OP, SELF_ARROW, JUST_C)); \
            } \
        } else if (PyObject_TypeCheck(obj, &Vn##Type)) { \
            Vn* other = (Vn*)obj; \
                result = PyObject_CallFunction((PyObject*)&Vn##Type, \
                        Vn##_REP("i"), \
                        Vn##_DO_OP(OP, SELF_ARROW, OTHER_ARROW)); \
        } \
        /* Leave `result` as NULL if the type is wrong. */ \
        \
        Py_XDECREF(obj); \
        return result; \
    }

#define ADD_OP(a, b) (a + b)
#define SUB_OP(a, b) (a - b)
#define MUL_OP(a, b) (a * b)
#define DIV_OP(a, b) (a < 0 ? (a - (b - 1)) / b : a / b)
#define MOD_OP(a, b) (a < 0 ? (a - (b - 1)) % b : a % b)

#define V2_DO_HASH          (MIX(self->x)); (MIX(self->y))
#define V3_DO_HASH          (MIX(self->x)); (MIX(self->y)); (MIX(self->z));

#define MIX(x)              hash = (hash << 4) ^ (hash >> 28) ^ (x)

#define GEN_HASH(Vn) \
    static long Vn##_hash(Vn* self) { \
        long hash = 0x6af5cd4dU; \
        Vn##_DO_HASH; \
        return hash; \
    }


static PyMemberDef V3_members[] = {
    {"x", T_INT, offsetof(V3, x), READONLY, NULL},
    {"y", T_INT, offsetof(V3, y), READONLY, NULL},
    {"z", T_INT, offsetof(V3, z), READONLY, NULL},
    {NULL}
};


static int V3_init(V3* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {"x", "y", "z", NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "iii", kwlist,
                &self->x, &self->y, &self->z)) {
        return -1;
    }
    return 0;
}

GEN_OP(V3, add, ADD_OP)
GEN_OP(V3, sub, SUB_OP)
GEN_OP(V3, mul, MUL_OP)
GEN_OP(V3, div, DIV_OP)
GEN_OP(V3, mod, MOD_OP)

GEN_HASH(V3)

static PyMethodDef V3_methods[] = {
    {"__add__", (PyCFunction)V3_add, METH_VARARGS, NULL},
    {"__sub__", (PyCFunction)V3_sub, METH_VARARGS, NULL},
    {"__mul__", (PyCFunction)V3_mul, METH_VARARGS, NULL},
    {"__div__", (PyCFunction)V3_div, METH_VARARGS, NULL},
    {"__mod__", (PyCFunction)V3_mod, METH_VARARGS, NULL},
    {"__hash__", (PyCFunction)V3_hash, METH_VARARGS, NULL},
    {NULL}
};

PyTypeObject V3Type = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.V3",
    sizeof(V3),
};

PyObject* v3_get_type() {
    V3Type.tp_flags = Py_TPFLAGS_DEFAULT;
    V3Type.tp_new = PyType_GenericNew;
    V3Type.tp_init = (initproc)V3_init;
    V3Type.tp_members = V3_members;
    V3Type.tp_methods = V3_methods;
    V3Type.tp_hash = (hashfunc)V3_hash;

    if (PyType_Ready(&V3Type) < 0) {
        return NULL;
    }
   
    return (PyObject*)&V3Type;
}

V3* v3_read(Reader* r) {
    struct {
        int32_t x;
        int32_t y;
        int32_t z;
    } val;
    READ(val);

    return (V3*)PyObject_CallFunction((PyObject*)&V3Type, "iii",
            val.x, val.y, val.z);

fail:
    return NULL;
}


static PyMemberDef V2_members[] = {
    {"x", T_INT, offsetof(V2, x), READONLY, NULL},
    {"y", T_INT, offsetof(V2, y), READONLY, NULL},
    {NULL}
};


static int V2_init(V2* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {"x", "y", NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "ii", kwlist,
                &self->x, &self->y)) {
        return -1;
    }
    return 0;
}

GEN_OP(V2, add, ADD_OP)
GEN_OP(V2, sub, SUB_OP)
GEN_OP(V2, mul, MUL_OP)
GEN_OP(V2, div, DIV_OP)
GEN_OP(V2, mod, MOD_OP)

GEN_HASH(V2)

static PyMethodDef V2_methods[] = {
    {"__add__", (PyCFunction)V2_add, METH_VARARGS, NULL},
    {"__sub__", (PyCFunction)V2_sub, METH_VARARGS, NULL},
    {"__mul__", (PyCFunction)V2_mul, METH_VARARGS, NULL},
    {"__div__", (PyCFunction)V2_div, METH_VARARGS, NULL},
    {"__mod__", (PyCFunction)V2_mod, METH_VARARGS, NULL},
    {NULL}
};

PyTypeObject V2Type = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.V2",
    sizeof(V2),
};

PyObject* v2_get_type() {
    V2Type.tp_flags = Py_TPFLAGS_DEFAULT;
    V2Type.tp_new = PyType_GenericNew;
    V2Type.tp_init = (initproc)V2_init;
    V2Type.tp_members = V2_members;
    V2Type.tp_methods = V2_methods;
    V2Type.tp_hash = (hashfunc)V2_hash;

    if (PyType_Ready(&V2Type) < 0) {
        return NULL;
    }
   
    return (PyObject*)&V2Type;
}

V2* v2_read(Reader* r) {
    struct {
        int32_t x;
        int32_t y;
    } val;
    READ(val);

    return (V2*)PyObject_CallFunction((PyObject*)&V2Type, "ii",
            val.x, val.y);

fail:
    return NULL;
}
