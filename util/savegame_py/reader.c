#include <Python.h>

#include "common.h"
#include "reader.h"


int reader_init(Reader* r, PyObject* bytes) {
    // NB: the PyObject* fields of `r` should all be null when this function is
    // called.
    r->data = PyBytes_AsString(bytes);
    r->offset = 0;
    r->len = PyBytes_Size(bytes);

    r->object_id_table = PyDict_New();
    FAIL_IF(r->object_id_table == NULL);

    r->item_name_table = PyDict_New();
    FAIL_IF(r->item_name_table == NULL);

    r->template_name_table = PyDict_New();
    FAIL_IF(r->template_name_table == NULL);

    return 0;

fail:
    SET_EXC();
    Py_XDECREF(r->object_id_table);
    Py_XDECREF(r->item_name_table);
    Py_XDECREF(r->template_name_table);
    return -1;
}

PyObject* read_decode_item_name(Reader* r, uint16_t old_id, size_t name_len) {
    PyObject* key = PyLong_FromLong(old_id);
    PyObject* value = NULL;
    FAIL_IF(key == NULL);

    value = PyDict_GetItem(r->item_name_table, key);
    if (value != NULL) {
        // NB: Not safe to `goto fail` in this case, because `value` is now a
        // borrowed reference.
        Py_INCREF(value);
        Py_DECREF(key);
        return value;
    } else {
        value = read_string(r, name_len);
        FAIL_IF(value == NULL);
        FAIL_IF(PyDict_SetItem(r->item_name_table, key, value) < 0);
        Py_DECREF(key);
        return value;
    }

fail:
    SET_EXC();
    Py_XDECREF(key);
    Py_XDECREF(value);
    return NULL;
}

PyObject* read_decode_template_name(Reader* r) {
    PyObject* key = NULL;
    PyObject* value = NULL;

    uint32_t old_id;
    READ(old_id);

    key = PyLong_FromUnsignedLong(old_id);
    FAIL_IF(key == NULL);

    value = PyDict_GetItem(r->template_name_table, key);
    if (value != NULL) {
        // NB: Not safe to `goto fail` in this case, because `value` is now a
        // borrowed reference.
        Py_INCREF(value);
        Py_DECREF(key);
        return value;
    } else {
        struct {
            uint8_t x;
            uint8_t y;
            uint8_t z;
            uint8_t name_len;
        } val;
        READ(val);

        value = read_string(r, val.name_len);
        FAIL_IF(value == NULL);
        FAIL_IF(PyDict_SetItem(r->template_name_table, key, value) < 0);
        Py_DECREF(key);
        return value;
    }

fail:
    SET_EXC();
    Py_XDECREF(key);
    Py_XDECREF(value);
    return NULL;
}

PyObject* read_string(Reader* r, size_t len) {
    PyObject* bytes = NULL;
    PyObject* str = NULL;

    bytes = PyByteArray_FromStringAndSize(NULL, 0);
    FAIL_IF(bytes == NULL);
    FAIL_IF(PyByteArray_Resize(bytes, len) < 0);
    FAIL_IF(read_bytes(r, PyByteArray_AsString(bytes), len) < 0);

    str = PyUnicode_FromEncodedObject(bytes, "utf-8", "strict");
    FAIL_IF(str == NULL);

    Py_XDECREF(bytes);
    return str;

fail:
    SET_EXC();
    Py_XDECREF(bytes);
    Py_XDECREF(str);
    return NULL;
}

int read_register_object(Reader* r, uint32_t save_id, PyObject* obj) {
    PyObject* key = PyLong_FromLong(save_id);
    FAIL_IF(key == NULL);

    FAIL_IF(PyDict_SetItem(r->object_id_table, key, obj) < 0);

    Py_XDECREF(key);
    return 0;

fail:
    SET_EXC();
    Py_XDECREF(key);
    return -1;
}

PyObject* read_find_object(Reader* r, uint32_t save_id) {
    PyObject* key = PyLong_FromLong(save_id);
    FAIL_IF(key == NULL);

    PyObject* result = PyDict_GetItem(r->object_id_table, key);
    FAIL_IF(result == NULL);

    Py_INCREF(result);
    Py_DECREF(key);
    return result;

fail:
    SET_EXC();
    Py_XDECREF(key);
    return NULL;
}
