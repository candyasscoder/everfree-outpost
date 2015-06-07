#ifndef OUTPOST_SAVEGAME_READER_H
#define OUTPOST_SAVEGAME_READER_H

#include <Python.h>
#include <string.h>

typedef struct {
    uint8_t* data;
    size_t offset;
    size_t len;
    PyObject* object_id_table;
    PyObject* item_name_table;
    PyObject* template_name_table;
} Reader;

static inline int read_bytes(Reader* r, void* buf, size_t len) {
    // Round up to the nearest multiple of 4 when advancing the offset.
    size_t padded_len = (len + 3) & ~3;

    if (r->len - r->offset < padded_len) {
        return -1;
    }
    memcpy(buf, r->data + r->offset, len);

    r->offset += padded_len;
    return 0;
}

#define READ(x)     FAIL_IF(read_bytes(r, &x, sizeof(x)) < 0)

int reader_init(Reader* r, PyObject* bytes);
PyObject* read_decode_item_name(Reader* r, uint16_t old_id, size_t name_len);
PyObject* read_decode_template_name(Reader* r);
PyObject* read_string(Reader* r, size_t len);
int read_register_object(Reader* r, uint32_t save_id, PyObject* obj);
PyObject* read_find_object(Reader* r, uint32_t save_id);

#endif // OUTPOST_SAVEGAME_READER_H
