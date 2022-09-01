#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#include "stdlib.h"

/*
  There are many tradeoffs in heaps. I think for Solidity, we want:
   - small code size to reduce code length
   - not many malloc objects
   - not much memory

  So I think we should avoid fragmentation by neighbour merging. The most
  costly is walking the doubly linked list looking for free space.
*/
struct chunk
{
    struct chunk *next, *prev;
    size_t length;
    size_t allocated;
};

void __init_heap()
{
    struct chunk *first = (struct chunk *)0x10000;
    first->next = first->prev = NULL;
    first->allocated = false;
    first->length = (size_t)(__builtin_wasm_memory_size(0) * 0x10000 -
                             (size_t)first - sizeof(struct chunk));
}

void __attribute__((noinline)) __free(void *m)
{
    struct chunk *cur = m;
    cur--;
    if (m)
    {
        cur->allocated = false;
        struct chunk *next = cur->next;
        if (next && !next->allocated)
        {
            // merge with next
            if ((cur->next = next->next) != NULL)
                cur->next->prev = cur;
            cur->length += next->length + sizeof(struct chunk);
            next = cur->next;
        }

        struct chunk *prev = cur->prev;
        if (prev && !prev->allocated)
        {
            // merge with previous
            prev->next = next;
            next->prev = prev;
            prev->length += cur->length + sizeof(struct chunk);
        }
    }
}

static void shrink_chunk(struct chunk *cur, size_t size)
{
    // round up to nearest 8 bytes
    size = (size + 7) & ~7;

    if (cur->length - size >= (8 + sizeof(struct chunk)))
    {
        // split and return
        void *data = (cur + 1);
        struct chunk *new = data + size;
        if ((new->next = cur->next) != NULL)
            new->next->prev = new;
        cur->next = new;
        new->prev = cur;
        new->allocated = false;
        new->length = cur->length - size - sizeof(struct chunk);
        cur->length = size;
    }
}

void *__attribute__((noinline)) __malloc(uint32_t size)
{
    struct chunk *cur = (struct chunk *)0x10000;

    while (cur && (cur->allocated || size > cur->length))
        cur = cur->next;

    if (cur)
    {
        shrink_chunk(cur, size);
        cur->allocated = true;
        return ++cur;
    }
    else
    {
        // go bang
        __builtin_unreachable();
    }
}

void *__realloc(void *m, size_t size)
{
    struct chunk *cur = m;

    cur--;

    struct chunk *next = cur->next;

    if (next && !next->allocated && size <= (cur->length + next->length + sizeof(struct chunk)))
    {
        // merge with next
        cur->next = next->next;
        cur->next->prev = cur;
        cur->length += next->length + sizeof(struct chunk);
        // resplit ..
        shrink_chunk(cur, size);
        return m;
    }
    else
    {
        // allocate new area and copy old data
        uint32_t len = cur->length;

        // if new size is smaller than the old data, only copy remaining data
        if (size < len)
            len = size;

        void *n = __malloc(size);

        // __memcpy8() copies 8 bytes at once; round up to the nearest 8 bytes
        // this is permitted because allocations are always aligned on 8 byte
        // boundaries anyway.
        __memcpy8(n, m, (len + 7) / 8);
        __free(m);
        return n;
    }
}
