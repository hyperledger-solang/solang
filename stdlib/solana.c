
#include <stdint.h>
#include <stddef.h>

#include "solana_sdk.h"

extern int solang_constructor(const uint8_t *input, uint64_t input_len, uint8_t *ouyput, uint64_t *output_len);
extern int solang_function(const uint8_t *input, uint64_t input_len, uint8_t *ouyput, uint64_t *output_len);

uint64_t
entrypoint(const uint8_t *input)
{
    SolAccountInfo ka[1];
    SolParameters params = (SolParameters){.ka = ka};
    if (!sol_deserialize(input, &params, SOL_ARRAY_SIZE(ka)))
    {
        return ERROR_INVALID_ARGUMENT;
    }

    if ((params.data_len % 32) == 0)
    {
        return solang_constructor(params.data, params.data_len, ka[0].data, ka[0].data_len);
    }
    else
    {
        return solang_function(params.data, params.data_len, ka[0].data, ka[0].data_len);
    }

    return SUCCESS;
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

    do
        *dest++ = 0;
    while (--length);
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
