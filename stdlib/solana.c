
#include <stdint.h>
#include <stddef.h>

#include "solana_sdk.h"

extern int solang_dispatch(const uint8_t *input, uint64_t input_len, SolAccountInfo *ka);

uint64_t
entrypoint(const uint8_t *input)
{
    SolAccountInfo ka[2];
    SolParameters params = (SolParameters){.ka = ka};
    if (!sol_deserialize(input, &params, SOL_ARRAY_SIZE(ka)))
    {
        return ERROR_INVALID_ARGUMENT;
    }

    return solang_dispatch(params.data, params.data_len, ka);
}

/*
 * Vector is used for dynamic array
 */
struct vector
{
    uint32_t len;
    uint32_t size;
    uint8_t data[];
};

void *__malloc(uint32_t size)
{
    return sol_alloc_free_(size, NULL);
}

/*
 * Fast-ish clear, 8 bytes at a time.
 */
void __bzero8(void *_dest, uint32_t length)
{
    uint64_t *dest = _dest;

    while (length--)
    {
        *dest++ = 0;
    }
}

void __memset8(void *_dest, uint64_t val, uint32_t length)
{
    uint64_t *dest = _dest;

    do
    {
        *dest++ = val;
    } while (--length);
}

void __memcpy(void *_dest, const void *_src, uint32_t length)
{
    uint8_t *dest = _dest;
    const uint8_t *src = _src;

    while (length--)
    {
        *dest++ = *src++;
    }
}

// Create a new vector. If initial is -1 then clear the data. This is done since a null pointer valid in wasm
struct vector *vector_new(uint32_t members, uint32_t size, uint8_t *initial)
{
    struct vector *v;
    uint32_t size_array = members * size;

    v = __malloc(sizeof(*v) + size_array);
    v->len = members;
    v->size = members;

    uint8_t *data = v->data;

    if ((int)initial != -1)
    {
        while (size_array--)
        {
            *data++ = *initial++;
        }
    }
    else
    {
        while (size_array--)
        {
            *data++ = 0;
        }
    }

    return v;
}

struct vector *concat(uint8_t *left, uint32_t left_len, uint8_t *right, uint32_t right_len)
{
    uint32_t size_array = left_len + right_len;
    struct vector *v = __malloc(sizeof(*v) + size_array);
    v->len = size_array;
    v->size = size_array;

    uint8_t *data = v->data;

    while (left_len--)
    {
        *data++ = *left++;
    }

    while (right_len--)
    {
        *data++ = *right++;
    }

    return v;
}

// This function is used for abi decoding integers.
// ABI encoding is big endian, and can have integers of 8 to 256 bits
// (1 to 32 bytes). This function copies length bytes and reverses the
// order since wasm is little endian.
void __be32toleN(uint8_t *from, uint8_t *to, uint32_t length)
{
    from += 31;

    do
    {
        *to++ = *from--;
    } while (--length);
}

void __beNtoleN(uint8_t *from, uint8_t *to, uint32_t length)
{
    from += length;

    do
    {
        *to++ = *--from;
    } while (--length);
}

void __leNtobeN(uint8_t *from, uint8_t *to, uint32_t length)
{
    to += length;

    do
    {
        *--to = *from++;
    } while (--length);
}

// This function is for used for abi encoding integers
// ABI encoding is big endian.
void __leNtobe32(uint8_t *from, uint8_t *to, uint32_t length)
{
    to += 31;

    do
    {
        *to-- = *from++;
    } while (--length);
}