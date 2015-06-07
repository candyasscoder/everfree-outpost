#include <Python.h>
#include <structmember.h>

#include "common.h"
#include "reader.h"


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

#define GEN_OP(name, OP) \
    static PyObject* V3_##name(V3* self, PyObject* args) { \
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
                result = PyObject_CallFunction((PyObject*)&V3Type, "iii", \
                        OP(self->x, c), \
                        OP(self->y, c), \
                        OP(self->z, c)); \
            } \
        } else if (PyObject_TypeCheck(obj, &V3Type)) { \
            V3* other = (V3*)obj; \
            result = PyObject_CallFunction((PyObject*)&V3Type, "iii", \
                    OP(self->x, other->x), \
                    OP(self->y, other->y), \
                    OP(self->z, other->z)); \
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

GEN_OP(add, ADD_OP)
GEN_OP(sub, SUB_OP)
GEN_OP(mul, MUL_OP)
GEN_OP(div, DIV_OP)
GEN_OP(mod, MOD_OP)

static PyMethodDef V3_methods[] = {
    {"__add__", (PyCFunction)V3_add, METH_VARARGS, NULL},
    {"__sub__", (PyCFunction)V3_sub, METH_VARARGS, NULL},
    {"__mul__", (PyCFunction)V3_mul, METH_VARARGS, NULL},
    {"__div__", (PyCFunction)V3_div, METH_VARARGS, NULL},
    {"__mod__", (PyCFunction)V3_mod, METH_VARARGS, NULL},
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
   
    if (PyType_Ready(&V3Type) < 0) {
        return NULL;
    }
   
    return (PyObject*)&V3Type;
}
