#include <stdint.h>
#include <stddef.h>

#include "stdlib.h"

extern uint32_t ext_get_storage(uint8_t *);
extern void ext_set_storage(uint8_t *, uint32_t value_non_null, uint8_t *, uint32_t);
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

uint8_t substrate_get_string_subscript(uint8_t *slot, uint32_t index)
{
    if (ext_get_storage(slot) || index >= ext_scratch_size())
    {
        // not in contract storage
        __builtin_unreachable();
    }
    else
    {
        uint8_t val;

        // if index is out of bounds, this will throw an error
        ext_scratch_read(&val, index, 1);

        return val;
    }
}

void substrate_set_string_subscript(uint8_t *slot, uint32_t index, int8_t val)
{
    if (ext_get_storage(slot))
    {
        // not in contract storage
        __builtin_unreachable();
    }

    uint32_t size = ext_scratch_size();

    if (index >= size)
    {
        // not in contract storage
        __builtin_unreachable();
    }

    uint8_t data[size];

    ext_scratch_read(data, 0, size);

    data[index] = val;

    ext_set_storage(slot, 1, data, size);
}

void substrate_bytes_push(uint8_t *slot, int8_t val)
{
    if (ext_get_storage(slot))
    {
        // not in contract storage
        __builtin_unreachable();
    }

    uint32_t size = ext_scratch_size();

    uint8_t data[size + 1];

    ext_scratch_read(data, 0, size);

    data[size] = val;

    ext_set_storage(slot, 1, data, size + 1);
}

uint8_t substrate_bytes_pop(uint8_t *slot)
{
    if (ext_get_storage(slot))
    {
        // not in contract storage
        __builtin_unreachable();
    }

    uint32_t size = ext_scratch_size();

    if (size == 0)
    {
        // nothing to pop off
        __builtin_unreachable();
    }

    uint8_t data[size];

    ext_scratch_read(data, 0, size);

    ext_set_storage(slot, 1, data, size - 1);

    return data[size - 1];
}

uint32_t substrate_string_length(uint8_t *slot)
{
    if (ext_get_storage(slot))
    {
        // not in contract storage
        return 0;
    }

    return ext_scratch_size();
}
