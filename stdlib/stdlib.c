// SPDX-License-Identifier: Apache-2.0

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#include "stdlib.h"

/*
 */
void __memset8(void *_dest, uint64_t val, uint32_t length)
{
    uint64_t *dest = _dest;

    do
    {
        *dest++ = val;
    } while (--length);
}

void __memset(void *_dest, uint8_t val, size_t length)
{
    uint8_t *dest = _dest;

    do
    {
        *dest++ = val;
    } while (--length);
}

/*
 * Our memcpy can only deal with multiples of 8 bytes. This is enough for
 * simple allocator below.
 */
void __memcpy8(void *_dest, void *_src, uint32_t length)
{
    uint64_t *dest = _dest;
    uint64_t *src = _src;

    do
    {
        *dest++ = *src++;
    } while (--length);
}

void *__memcpy(void *_dest, const void *_src, uint32_t length)
{
    uint8_t *dest = _dest;
    const uint8_t *src = _src;

    while (length--)
    {
        *dest++ = *src++;
    }

    return dest;
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

int __memcmp_ord(uint8_t *a, uint8_t *b, uint32_t len)
{
    do
    {
        int diff = (int)(*a++) - (int)(*b++);

        if (diff)
            return diff;
    } while (--len);

    return 0;
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

void __leNtobeN(uint8_t *from, uint8_t *to, uint32_t length)
{
    to += length;
    do
    {
        *--to = *from++;
    } while (--length);
}

uint64_t vector_hash(struct vector *v)
{
    uint64_t hash = 0;
    uint8_t *data = v->data;
    uint32_t len = v->len;

    while (len--)
    {
        hash += *data;
    }

    return hash;
}

bool __memcmp(uint8_t *left, uint32_t left_len, uint8_t *right, uint32_t right_len)
{
    if (left_len != right_len)
        return false;

    while (left_len--)
    {
        if (*left++ != *right++)
            return false;
    }

    return true;
}

#ifndef TEST

#ifdef __wasm__
#define VECTOR_EMPTY ((uint8_t *)~0l)
#else
#define VECTOR_EMPTY ((uint8_t *)0l)
#endif

// Create a new vector. If initial is -1 then clear the data. This is done since a null pointer is valid in Wasm
struct vector *vector_new(uint32_t members, uint32_t size, uint8_t *initial)
{
    struct vector *v;
    uint32_t size_array = members * size;

    v = __malloc(sizeof(*v) + size_array);
    v->len = members;
    v->size = members;

    uint8_t *data = v->data;

    if (initial != VECTOR_EMPTY)
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

#endif
