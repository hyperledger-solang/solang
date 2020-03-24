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

// 32 bit integer as as scale compact integer
// https://substrate.dev/docs/en/conceptual/core/codec#vectors-lists-series-sets
uint8_t *compact_encode_u32(uint8_t *dest, uint32_t val)
{
    if (val < 64)
    {
        *dest++ = val << 2;
    }
    else if (val < 0x4000)
    {
        *((uint16_t *)dest) = (val << 2) | 1;
        dest += 2;
    }
    else if (val < 0x40000000)
    {
        *((uint32_t *)dest) = (val << 2) | 2;
        dest += 4;
    }
    else
    {
        *dest++ = 3;
        *((uint32_t *)dest) = val;
        dest += 4;
    }

    return dest;
}

uint8_t *compact_decode_u32(uint8_t *dest, uint32_t *val)
{
    switch (*dest & 3)
    {
    case 0:
        *val = *dest >> 2;
        dest += 1;
        break;
    case 1:
        *val = *((uint16_t *)dest) >> 2;
        dest += 2;
        break;
    case 2:
        *val = *((uint32_t *)dest) >> 2;
        dest += 4;
        break;
    default:
        // sizes of 2**30 (1GB) or larger are not allowed
        __builtin_unreachable();
    }

    return dest;
}

uint8_t *scale_encode_string(uint8_t *dest, struct vector *s)
{
    uint32_t len = s->len;
    uint8_t *data_dst = compact_encode_u32(dest, len);
    uint8_t *data = s->data;

    while (len--)
    {
        *data_dst++ = *data++;
    }

    return data_dst;
}

struct vector *scale_decode_string(uint8_t **from)
{
    uint8_t *src = *from;
    uint32_t size_array;

    src = compact_decode_u32(src, &size_array);

    struct vector *v = __malloc(sizeof(*v) + size_array);

    v->len = size_array;
    v->size = size_array;

    uint8_t *data = v->data;

    while (size_array--)
    {
        *data++ = *src++;
    }

    *from = src;

    return v;
}
