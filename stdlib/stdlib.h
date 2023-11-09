// SPDX-License-Identifier: Apache-2.0

/*
 * Vector is used for dynamic array
 */
struct vector
{
    uint32_t len;
    uint32_t size;
    uint8_t data[];
};

extern void *__malloc(uint32_t size);
extern void __memset(void *dest, uint8_t val, size_t length);
extern void *__memcpy(void *dest, const void *src, uint32_t length);
extern void __memcpy8(void *_dest, void *_src, uint32_t length);
