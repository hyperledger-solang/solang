#include <stdint.h>
#include <stddef.h>

#include "stdlib.h"

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
