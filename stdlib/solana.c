
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

void __mul32(uint32_t left[], uint32_t right[], uint32_t out[], int len)
{
    uint64_t val1 = 0, carry = 0;

    int left_len = len, right_len = len;

    while (left_len > 0 && !left[left_len - 1])
        left_len--;

    while (right_len > 0 && !right[right_len - 1])
        right_len--;

    int right_start = 0, right_end = 0;
    int left_start = 0;

    for (int l = 0; l < len; l++)
    {
        int i = 0;

        if (l >= left_len)
            right_start++;

        if (l >= right_len)
            left_start++;

        if (right_end < right_len)
            right_end++;

        for (int r = right_end - 1; r >= right_start; r--)
        {
            uint64_t m = (uint64_t)left[left_start + i] * (uint64_t)right[r];
            i++;
            if (__builtin_add_overflow(val1, m, &val1))
                carry += 0x100000000;
        }

        out[l] = val1;

        val1 = (val1 >> 32) | carry;
        carry = 0;
    }
}

// Some compiler runtime builtins we need.

// 128 bit shift left.
typedef union
{
    __uint128_t all;
    struct
    {
        uint64_t low;
        uint64_t high;
    };
} two64;

// 128 bit shift left.
typedef union
{
    __int128_t all;
    struct
    {
        uint64_t low;
        int64_t high;
    };
} two64s;

// This assumes r >= 0 && r <= 127
__uint128_t __ashlti3(__uint128_t val, int r)
{
    two64 in;
    two64 result;

    in.all = val;

    if (r == 0)
    {
        // nothing to do
        result.all = in.all;
    }
    else if (r & 64)
    {
        // Shift more than or equal 64
        result.low = 0;
        result.high = in.low << (r & 63);
    }
    else
    {
        // Shift less than 64
        result.low = in.low << r;
        result.high = (in.high << r) | (in.low >> (64 - r));
    }

    return result.all;
}

// This assumes r >= 0 && r <= 127
__uint128_t __lshrti3(__uint128_t val, int r)
{
    two64 in;
    two64 result;

    in.all = val;

    if (r == 0)
    {
        // nothing to do
        result.all = in.all;
    }
    else if (r & 64)
    {
        // Shift more than or equal 64
        result.low = in.high >> (r & 63);
        result.high = 0;
    }
    else
    {
        // Shift less than 64
        result.low = (in.low >> r) | (in.high << (64 - r));
        result.high = in.high >> r;
    }

    return result.all;
}

__uint128_t __ashrti3(__uint128_t val, int r)
{
    two64s in;
    two64s result;

    in.all = val;

    if (r == 0)
    {
        // nothing to do
        result.all = in.all;
    }
    else if (r & 64)
    {
        // Shift more than or equal 64
        result.high = in.high >> 63;
        result.low = in.high >> (r & 63);
    }
    else
    {
        // Shift less than 64
        result.low = (in.low >> r) | (in.high << (64 - r));
        result.high = in.high >> r;
    }

    return result.all;
}
