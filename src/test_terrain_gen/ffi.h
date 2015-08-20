#ifndef OUTPOST_TEST_TERRAIN_GEN_FFI_INCLUDED
#define OUTPOST_TEST_TERRAIN_GEN_FFI_INCLUDED

#include <stddef.h>
#include <stdint.h>

typedef struct tg_worker tg_worker;
typedef struct tg_chunk tg_chunk;
typedef struct tg_structure tg_structure;
typedef struct tg_extra_iter tg_extra_iter;

typedef uint16_t block_id;
typedef uint32_t template_id;

tg_worker* worker_create(const char* path);
void worker_destroy(tg_worker* w);
void worker_request(tg_worker* w, uint64_t pid, int32_t x, int32_t y);
tg_chunk* worker_get_response(tg_worker* w, uint64_t* pid_p, int32_t* x_p, int32_t* y_p);

void chunk_free(tg_chunk* c);
size_t chunk_blocks_len(const tg_chunk* c);
block_id chunk_get_block(const tg_chunk* c, size_t idx);
size_t chunk_structures_len(const tg_chunk* c);
const tg_structure* chunk_get_structure(const tg_chunk* c, size_t idx);

void structure_get_pos(const tg_structure* s, int32_t* x_p, int32_t* y_p, int32_t* z_p);
template_id structure_get_template(const tg_structure* s);
size_t structure_extra_len(const tg_structure* s);
tg_extra_iter* structure_extra_iter(const tg_structure* s);

void extra_iter_free(tg_extra_iter* i);
int extra_iter_next(tg_extra_iter* i,
        const char** key_p, size_t* key_len_p,
        const char** value_p, size_t* value_len_p);

#endif // OUTPOST_TEST_TERRAIN_GEN_FFI_INCLUDED
