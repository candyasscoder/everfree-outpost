#include <Python.h>
#include <structmember.h>

#include "entity.h"
#include "inventory.h"

#include "common.h"
#include "extra.h"
#include "object_id.h"
#include "reader.h"

typedef struct {
    uint32_t save_id;
    PyObject* extra_raw;
} EntitySave;

typedef struct _Entity {
    PyObject_HEAD

    int version;
    EntitySave* save;
    uint64_t stable_id;
    PyObject* extra;

    PyObject* stable_plane;
    PyObject* motion;
    uint16_t anim;
    PyObject* facing;
    PyObject* target_velocity;
    uint32_t appearance;

    PyObject* child_inventories;
} Entity;

static PyTypeObject EntityType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.Entity",
    sizeof(Entity),
};

static PyMemberDef Entity_members[] = {
    {"version", T_INT, offsetof(Entity, version), 0, NULL},
    {"stable_id", T_ULONGLONG, offsetof(Entity, stable_id), 0, NULL},
    {"extra", T_OBJECT, offsetof(Entity, extra), 0, NULL},
    {"stable_plane", T_OBJECT, offsetof(Entity, stable_plane), 0, NULL},
    {"motion", T_OBJECT, offsetof(Entity, motion), 0, NULL},
    {"anim", T_USHORT, offsetof(Entity, anim), 0, NULL},
    {"facing", T_OBJECT, offsetof(Entity, facing), 0, NULL},
    {"target_velocity", T_OBJECT, offsetof(Entity, target_velocity), 0, NULL},
    {"appearance", T_UINT, offsetof(Entity, appearance), 0, NULL},
    {"child_inventories", T_OBJECT, offsetof(Entity, child_inventories), 0, NULL},
    {NULL}
};

static void Entity_dealloc(Entity* self) {
    Py_XDECREF(self->extra);
    Py_XDECREF(self->stable_plane);
    Py_XDECREF(self->motion);
    Py_XDECREF(self->facing);
    Py_XDECREF(self->target_velocity);
    Py_XDECREF(self->child_inventories);

    if (self->save != NULL) {
        Py_XDECREF(self->save->extra_raw);
    }
    free(self->save);
}

static int Entity_init(Entity* self, PyObject* args, PyObject* kwds) {
    static char* kwlist[] = {NULL};
    if (!PyArg_ParseTupleAndKeywords(args, kwds, "", kwlist)) {
        return -1;
    }

    self->child_inventories = PyList_New(0);
    if (self->child_inventories == NULL) {
        goto fail;
    }

    return 0;

fail:
    Py_XDECREF(self->child_inventories);
    return -1;
}

PyObject* entity_get_type() {
    EntityType.tp_flags = Py_TPFLAGS_DEFAULT;
    EntityType.tp_new = PyType_GenericNew;
    EntityType.tp_dealloc = (destructor)Entity_dealloc;
    EntityType.tp_init = (initproc)Entity_init;
    EntityType.tp_members = Entity_members;

    if (PyType_Ready(&EntityType) < 0) {
        return NULL;
    }

    return (PyObject*)&EntityType;
}


typedef struct _Motion {
    PyObject_HEAD

    int64_t start_time;
    uint16_t duration;
    PyObject* start_pos;
    PyObject* end_pos;
} Motion;

static PyTypeObject MotionType = {
    PyVarObject_HEAD_INIT(NULL, 0)
    "outpost_savegame.Motion",
    sizeof(Motion),
};

static PyMemberDef Motion_members[] = {
    {"start_time", T_LONGLONG, offsetof(Motion, start_time), 0, NULL},
    {"duration", T_USHORT, offsetof(Motion, duration), 0, NULL},
    {"start_pos", T_OBJECT, offsetof(Motion, start_time), 0, NULL},
    {"end_pos", T_OBJECT, offsetof(Motion, end_pos), 0, NULL},
    {NULL}
};

static void Motion_dealloc(Motion* self) {
    Py_XDECREF(self->start_pos);
    Py_XDECREF(self->end_pos);
}

PyObject* motion_get_type() {
    MotionType.tp_flags = Py_TPFLAGS_DEFAULT;
    MotionType.tp_new = PyType_GenericNew;
    MotionType.tp_dealloc = (destructor)Motion_dealloc;
    MotionType.tp_members = Motion_members;

    if (PyType_Ready(&MotionType) < 0) {
        return NULL;
    }

    return (PyObject*)&MotionType;
}


Entity* entity_read(Reader* r, int version) {
    printf("begin reading entity\n");
    Entity* e = (Entity*)PyObject_CallObject((PyObject*)&EntityType, NULL);
    FAIL_IF(e == NULL);

    e->version = version;
    e->save = calloc(sizeof(EntitySave), 1);
    READ(e->save->save_id);
    FAIL_IF(read_register_object(r, e->save->save_id, (PyObject*)e) < 0);
    READ(e->stable_id);


    struct V3Data {
        int32_t x;
        int32_t y;
        int32_t z;
    };

    struct {
        uint64_t stable_plane;
        struct V3Data start_pos;
        struct V3Data end_pos;
        int64_t start_time;
        uint16_t duration;
        uint16_t anim;
        struct V3Data facing;
        struct V3Data target_velocity;
        uint32_t appearance;
    } data;
    READ(data);

    e->stable_plane = PyObject_CallFunction((PyObject*)&StablePlaneIdType, "K", data.stable_plane);
    FAIL_IF(e->stable_plane == NULL);

    e->motion = PyObject_CallObject((PyObject*)&MotionType, NULL);
    FAIL_IF(e->motion == NULL);
    Motion* motion = (Motion*)e->motion;
    motion->start_time = data.start_time;
    motion->duration = data.duration;
    motion->start_pos = PyObject_CallFunction((PyObject*)&V3Type, "iii",
            data.start_pos.x, data.start_pos.y, data.start_pos.z);
    FAIL_IF(motion->start_pos == NULL);
    motion->end_pos = PyObject_CallFunction((PyObject*)&V3Type, "iii",
            data.end_pos.x, data.end_pos.y, data.end_pos.z);
    FAIL_IF(motion->end_pos == NULL);

    e->anim = data.anim;

    e->facing = PyObject_CallFunction((PyObject*)&V3Type, "iii",
            data.facing.x, data.facing.y, data.facing.z);
    FAIL_IF(e->facing == NULL);

    e->target_velocity = PyObject_CallFunction((PyObject*)&V3Type, "iii",
            data.target_velocity.x, data.target_velocity.y, data.target_velocity.z);
    FAIL_IF(e->facing == NULL);

    e->appearance = data.appearance;


    e->save->extra_raw = extra_read(r, version);
    FAIL_IF(e->save->extra_raw  == NULL);


    uint32_t count;

    READ(count);
    printf("  read %d inventories for entity\n", count);
    for (uint32_t i = 0; i < count; ++i) {
        Inventory* obj = inventory_read(r, version);
        FAIL_IF(obj == NULL);
        FAIL_IF(PyList_Append(e->child_inventories, (PyObject*)obj) == -1);
    }

    return e;

fail:
    Py_XDECREF(e);
    return NULL;
}

int entity_read_post(Reader* r, Entity* e, int version) {
    e->extra = extra_read_post(r, e->save->extra_raw, version);
    FAIL_IF(e->extra == NULL);

    Py_DECREF(e->save->extra_raw);
    free(e->save);
    e->save = NULL;


    Py_ssize_t len;
    len = PyList_Size(e->child_inventories);
    for (Py_ssize_t i = 0; i < len; ++i) {
        PyObject* item = PyList_GetItem(e->child_inventories, i);
        FAIL_IF(item == NULL);
        FAIL_IF(inventory_read_post(r, (Inventory*)item, version) < 0);
    }

    return 0;

fail:
    return -1;
}
