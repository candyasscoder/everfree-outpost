#ifndef OUTPOST_SAVEGAME_COMMON_H
#define OUTPOST_SAVEGAME_COMMON_H

#include <Python.h>

#include "reader.h"


#define FAIL_IF(c) \
    do { \
        if (c) { \
            goto fail; \
        } \
    } while(0)


typedef struct _V3 {
    PyObject_HEAD
    int32_t x;
    int32_t y;
    int32_t z;
} V3;

extern PyTypeObject V3Type;

PyObject* v3_get_type();
V3* v3_read(Reader* r);


typedef struct _V2 {
    PyObject_HEAD
    int32_t x;
    int32_t y;
} V2;

extern PyTypeObject V2Type;

PyObject* v2_get_type();
V2* v2_read(Reader* r);

#endif // OUTPOST_SAVEGAME_COMMON_H
