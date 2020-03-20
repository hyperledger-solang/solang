#include <stdint.h>
#include <stddef.h>

#include "stdlib.h"

extern uint32_t ext_get_storage(uint8_t *);
extern uint32_t ext_scratch_size(void);
extern void ext_scratch_read(uint8_t *dest, uint32_t offset, uint32_t size);

struct vector *substrate_get_string(uint8_t *slot)
{
    struct vector *v;

    if (ext_get_storage(slot))
    {
        v = __malloc(sizeof(*v) + 0);
        v->size = 0;
        v->len = 0;
    }
    else
    {
        uint32_t size = ext_scratch_size();
        v = __malloc(sizeof(*v) + size);
        v->size = size;
        v->len = size;

        ext_scratch_read(v->data, 0, size);
    }

    return v;
}