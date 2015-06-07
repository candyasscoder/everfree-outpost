#include <Python.h>
#include <structmember.h>
#include <limits.h>

#include "client.h"
#include "common.h"
#include "object_id.h"
#include "reader.h"



enum Tag {
    NIL = 0x00,
    BOOL,
    SMALL_INT,
    LARGE_INT,
    FLOAT,
    SMALL_STRING,
    LARGE_STRING,
    TABLE,

    WORLD = 0x10,
    CLIENT,
    ENTITY,
    INVENTORY,
    STRUCTURE,

    STABLE_CLIENT = 0x20,
    STABLE_ENTITY,
    STABLE_INVENTORY,
    STABLE_PLANE,
    STABLE_STRUCTURE,

    T_V3 = 0x30,
    TIME_U,
};

PyObject* read_table(Reader* r, int version);

PyObject* extra_read(Reader* r, int version) {
    printf("begin reading extra\n");
    struct {
        uint8_t tag;
        uint8_t a;
        uint16_t b;
    } x;
    READ(x);
    printf("  tag = %x\n", x.tag);
    PyObject* result = NULL;

    // NB: Most of the simple cases omit `result == NULL` checks.  If `result`
    // is NULL, Py_XDECREF(result) is a no-op, and `return result` is the same
    // as `return NULL`.
    switch (x.tag) {
        case NIL:
            Py_INCREF(Py_None);
            return Py_None;

        case BOOL:
            if (x.a) {
                Py_INCREF(Py_True);
                return Py_True;
            } else {
                Py_INCREF(Py_False);
                return Py_False;
            }

        case SMALL_INT:
            result = PyLong_FromLong((int16_t)x.b);
            break;

        case LARGE_INT: {
            int32_t val;
            READ(val);
            result = PyLong_FromLong(val);
            break;
        }

        case FLOAT: {
            double val;
            READ(val);
            result = PyFloat_FromDouble(val);
            break;
        }

        case SMALL_STRING:
            result = read_string(r, x.b);
            break;

        case LARGE_STRING: {
            uint32_t len;
            READ(len);
            result = read_string(r, len);
            break;
        }

        case TABLE:
            result = read_table(r, version);
            break;


        case WORLD:
            result = PyObject_CallObject((PyObject*)&WorldType, NULL);
            break;

        case CLIENT:
            result = object_id_read(r, &ClientIdType);
            break;

        case ENTITY:
            result = object_id_read(r, &EntityIdType);
            break;

        case INVENTORY:
            result = object_id_read(r, &InventoryIdType);
            break;

        case STRUCTURE:
            result = object_id_read(r, &StructureIdType);
            break;


        case STABLE_CLIENT:
            result = stable_id_read(r, &StableClientIdType);
            break;

        case STABLE_ENTITY:
            result = stable_id_read(r, &StableEntityIdType);
            break;

        case STABLE_INVENTORY:
            result = stable_id_read(r, &StableInventoryIdType);
            break;

        case STABLE_PLANE:
            result = stable_id_read(r, &StablePlaneIdType);
            break;

        case STABLE_STRUCTURE:
            result = stable_id_read(r, &StableStructureIdType);
            break;


        case T_V3: {
            struct {
                int32_t x;
                int32_t y;
                int32_t z;
            } val;
            READ(val);
            result = PyObject_CallFunction((PyObject*)&V3Type, "iii", val.x, val.y, val.z);
            break;
        }

        case TIME_U: {
            uint64_t val;
            READ(val);
            result = PyLong_FromUnsignedLongLong(val);
            break;
        }

    }

    return result;

fail:
    Py_XDECREF(result);
    return NULL;
}

PyObject* read_table(Reader* r, int version) {
    PyObject* dct = PyDict_New();
    PyObject* key = NULL;
    PyObject* value = NULL;

    for (;;) {
        key = extra_read(r, version);
        if (key == NULL) {
            goto fail;
        }

        if (key == Py_None) {
            Py_DECREF(key);
            key = NULL;
            break;
        }

        value = extra_read(r, version);
        if (value == NULL) {
            Py_DECREF(key);
            goto fail;
        }

        int ret = PyDict_SetItem(dct, key, value);
        Py_DECREF(key);
        Py_DECREF(value);
        if (ret < 0) {
            goto fail;
        }
    }

    return dct;

fail:
    Py_XDECREF(dct);
    return NULL;
}


static int is_listlike_dict(PyObject* dct) {
    int64_t min = INT64_MAX;
    int64_t max = INT64_MIN;
    size_t count = 0;

    PyObject *key, *value;
    Py_ssize_t pos = 0;
    while (PyDict_Next(dct, &pos, &key, &value)) {
        if (!PyLong_Check(key)) {
            return 0;
        }

        int overflow = 0;
        int64_t cur = PyLong_AsLongLongAndOverflow(key, &overflow);
        if (overflow) {
            return 0;
        }
        if (cur < min) {
            min = cur;
        }
        if (cur > max) {
            max = cur;
        }
        ++count;
    }

    // Lua lists are 1-based.
    return min == 1 && max == count;
}

PyObject* extra_read_post(Reader* r, PyObject* extra, int version) {
    PyObject* result = NULL;
    if (PyDict_Check(extra)) {
        if (is_listlike_dict(extra)) {
            result = PyList_New(PyDict_Size(extra));

            PyObject *key, *value;
            Py_ssize_t pos = 0;
            while (PyDict_Next(extra, &pos, &key, &value)) {
                // is_listlike_dict already checked for overflow.
                int64_t idx = PyLong_AsLongLong(key);
                PyObject* new_value = extra_read_post(r, value, version);
                FAIL_IF(new_value == NULL);

                // Subtract 1 to adjust for Lua's 1-based lists.
                if (PyList_SetItem(result, idx - 1, new_value) < 0) {
                    Py_DECREF(new_value);
                    goto fail;
                }
            }
        } else {
            result = PyDict_New();

            PyObject *key, *value;
            Py_ssize_t pos = 0;
            while (PyDict_Next(extra, &pos, &key, &value)) {
                PyObject* new_value = extra_read_post(r, value, version);
                FAIL_IF(new_value == NULL);

                // Subtract 1 to adjust for Lua's 1-based lists.
                if (PyDict_SetItem(result, key, new_value) < 0) {
                    Py_DECREF(new_value);
                    goto fail;
                }
            }
        }
    } else if (PyObject_TypeCheck(extra, &ClientIdType) ||
            PyObject_TypeCheck(extra, &EntityIdType) ||
            PyObject_TypeCheck(extra, &InventoryIdType) ||
            PyObject_TypeCheck(extra, &StructureIdType)) {
        uint32_t id = ((AnyId*)extra)->id;
        result = read_find_object(r, id);
        FAIL_IF(result == NULL);
        Py_INCREF(result);
    } else {
        result = extra;
        Py_INCREF(result);
    }

    return result;

fail:
    Py_XDECREF(result);
    return NULL;
}
