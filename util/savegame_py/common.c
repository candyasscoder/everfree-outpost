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
    static PyObject* Vn##_##name(PyObject* obj1, PyObject* obj2) { \
        if (!PyObject_TypeCheck(obj1, &Vn##Type)) { \
            return NULL; \
        } \
        Vn* self = (Vn*)obj1; \
        \
        PyObject* result = NULL; \
        \
        if (PyLong_Check(obj2)) { \
            int overflow = 0; \
            int32_t c = PyLong_AsLongAndOverflow(obj2, &overflow); \
            if (!overflow) { \
                result = PyObject_CallFunction((PyObject*)&Vn##Type, \
                        Vn##_REP("i"), \
                        Vn##_DO_OP(OP, SELF_ARROW, JUST_C)); \
            } \
        } else if (PyObject_TypeCheck(obj2, &Vn##Type)) { \
            Vn* other = (Vn*)obj2; \
            result = PyObject_CallFunction((PyObject*)&Vn##Type, \
                    Vn##_REP("i"), \
                    Vn##_DO_OP(OP, SELF_ARROW, OTHER_ARROW)); \
        } \
        /* Leave `result` as NULL if the type is wrong. */ \
        \
        return result; \
    }

#define ADD_OP(a, b) (a + b)
#define SUB_OP(a, b) (a - b)
#define MUL_OP(a, b) (a * b)

#define GEN_DIVMOD(Vn) \
    static PyObject* Vn##_divmod(PyObject* obj1, PyObject* obj2) { \
        if (!PyObject_TypeCheck(obj1, &Vn##Type)) { \
            return NULL; \
        } \
        Vn* self = (Vn*)obj1; \
        \
        PyObject* div = NULL; \
        PyObject* mod = NULL; \
        \
        if (PyLong_Check(obj2)) { \
            int overflow = 0; \
            int32_t c = PyLong_AsLongAndOverflow(obj2, &overflow); \
            if (overflow) { \
                return NULL; \
            } \
            div = PyObject_CallFunction((PyObject*)&Vn##Type, \
                    Vn##_REP("i"), \
                    Vn##_DO_OP(DIV_OP, SELF_ARROW, JUST_C)); \
            if (div != NULL) { \
                mod = PyObject_CallFunction((PyObject*)&Vn##Type, \
                        Vn##_REP("i"), \
                        Vn##_DO_OP(MOD_OP, SELF_ARROW, JUST_C)); \
            } \
        } else if (PyObject_TypeCheck(obj2, &Vn##Type)) { \
            Vn* other = (Vn*)obj2; \
            div = PyObject_CallFunction((PyObject*)&Vn##Type, \
                    Vn##_REP("i"), \
                    Vn##_DO_OP(DIV_OP, SELF_ARROW, OTHER_ARROW)); \
            if (div != NULL) { \
                mod = PyObject_CallFunction((PyObject*)&Vn##Type, \
                        Vn##_REP("i"), \
                        Vn##_DO_OP(MOD_OP, SELF_ARROW, OTHER_ARROW)); \
            } \
        } \
        if (div != NULL && mod != NULL) { \
            return Py_BuildValue("OO", div, mod); \
        } else { \
            Py_XDECREF(div); \
            Py_XDECREF(mod); \
            return NULL; \
        } \
    }

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

#define V2_DO_COMP(OP, LHS, RHS) \
    ((LHS(x) OP RHS(x)) && \
     (LHS(y) OP RHS(y)))
#define V3_DO_COMP(OP, LHS, RHS) \
    ((LHS(x) OP RHS(x)) && \
     (LHS(y) OP RHS(y)) && \
     (LHS(z) OP RHS(z)))

#define V2_DO(m)        m(x) m(y)
#define V3_DO(m)        m(x) m(y) m(z)

#define DECL_OTHER_TEMP(X)              int32_t other_##X = 0;
#define INIT_OTHER_TEMP_Vn(X)           other_##X = other->X;
#define INIT_OTHER_TEMP_CONST(X)        other_##X = c;

#define OTHER_TEMP(X)       other_##X

#define GEN_COMP(Vn) \
    static PyObject* Vn##_richcompare(PyObject* obj1, PyObject* obj2, int op) { \
        /* Get `self` */ \
        if (!PyObject_TypeCheck(obj1, &Vn##Type)) { \
            return NULL; \
        } \
        Vn* self = (Vn*)obj1; \
        \
        /* Get `other`.  Use temporaries other_x, other_y, other_z so we can
         * allow `other` to be either a vector or a number. */ \
        Vn##_DO(DECL_OTHER_TEMP) \
        if (PyObject_TypeCheck(obj2, &Vn##Type)) { \
            Vn* other = (Vn*)obj2; \
            Vn##_DO(INIT_OTHER_TEMP_Vn); \
        } else if (PyLong_Check(obj2)) { \
            int overflow = 0; \
            int32_t c = PyLong_AsLongAndOverflow(obj2, &overflow); \
            if (overflow) { \
                return NULL; \
            } \
            Vn##_DO(INIT_OTHER_TEMP_CONST); \
        } else { \
            return NULL; \
        } \
        \
        switch (op) { \
            case Py_EQ: \
                return PyBool_FromLong(Vn##_DO_COMP(==, SELF_ARROW, OTHER_TEMP)); \
            case Py_NE: \
                return PyBool_FromLong(!Vn##_DO_COMP(==, SELF_ARROW, OTHER_TEMP)); \
            default: \
                Py_INCREF(Py_NotImplemented); \
                return Py_NotImplemented; \
        } \
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
GEN_DIVMOD(V3)

GEN_HASH(V3)
GEN_COMP(V3)

static PyMethodDef V3_methods[] = {
    {NULL}
};

static PyNumberMethods V3_number_methods = {0};

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
    V3Type.tp_richcompare = (richcmpfunc)V3_richcompare;
    V3Type.tp_as_number = &V3_number_methods;

    V3_number_methods.nb_add = (binaryfunc)V3_add;
    V3_number_methods.nb_subtract = (binaryfunc)V3_sub;
    V3_number_methods.nb_multiply = (binaryfunc)V3_mul;
    V3_number_methods.nb_divmod = V3_divmod;

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
    SET_EXC();
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
GEN_DIVMOD(V2)

GEN_HASH(V2)
GEN_COMP(V2)

static PyMethodDef V2_methods[] = {
    {NULL}
};

PyNumberMethods V2_number_methods = {0};

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
    V2Type.tp_richcompare = (richcmpfunc)V2_richcompare;
    V2Type.tp_as_number = &V2_number_methods;

    V2_number_methods.nb_add = (binaryfunc)V2_add;
    V2_number_methods.nb_subtract = (binaryfunc)V2_sub;
    V2_number_methods.nb_multiply = (binaryfunc)V2_mul;
    V2_number_methods.nb_divmod = V2_divmod;

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
    SET_EXC();
    return NULL;
}
