
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

void *__malloc(uint32_t size)
{
    return sol_alloc_free_(size, NULL);
}

struct account_data_header
{
    uint32_t magic;
    uint32_t heap_offset;
};

// Simple heap for account data
//
// The heap is a doubly-linked list of objects, so we can merge with neighbours when we free.
// We should use offsets rather than pointers as the layout in memory will be different each
// time it is called.
// We don't expect the account data to exceed 4GB so we use 32 bit offsets.
// The account data can grow so the last entry always has length = 0 and offset_next = 0.
struct chunk
{
    uint32_t offset_next, offset_prev;
    uint32_t length;
    uint32_t allocated;
};

#define ROUND_UP(n, d) (((n) + (d)-1) & ~(d - 1))

uint32_t account_data_alloc(SolAccountInfo *ai, uint32_t size)
{
    void *data = ai->data;
    struct account_data_header *hdr = data;

    if (!size)
        return 0;

    uint32_t offset = hdr->heap_offset;

    uint32_t alloc_size = ROUND_UP(size, 8);

    uint32_t offset_prev = 0;

    for (;;)
    {
        struct chunk *chunk = data + offset;

        if (!chunk->allocated)
        {
            if (!chunk->length)
            {
                offset += sizeof(struct chunk);

                uint32_t length = ai->data_len - offset;

                if (length < alloc_size)
                {
                    sol_log("account does not have enough storage");
                    sol_panic();
                }

                chunk->offset_next = offset + alloc_size;
                chunk->offset_prev = offset_prev;
                chunk->allocated = true;
                chunk->length = size;

                struct chunk *next = data + chunk->offset_next;

                next->offset_prev = offset - sizeof(struct chunk);
                next->length = 0;
                next->offset_next = 0;
                next->allocated = false;

                return offset;
            }
            else if (chunk->length < alloc_size)
            {
                // too small
            }
            else if (alloc_size + sizeof(struct chunk) + 8 > chunk->length)
            {
                // just right
                chunk->allocated = true;
                chunk->length = size;

                return offset + sizeof(struct chunk);
            }
            else
            {
                // too big, split
                uint32_t next = chunk->offset_next;
                uint32_t prev = offset;

                uint32_t next_offset = offset + sizeof(struct chunk) + alloc_size;

                chunk->offset_next = next_offset;
                chunk->length = size;
                chunk->allocated = true;

                chunk = data + next_offset;
                chunk->offset_prev = prev;
                chunk->offset_next = next;
                chunk->length = next - next_offset - sizeof(struct chunk);
                chunk->allocated = false;

                if (next)
                {
                    struct chunk *chunk = data + next;
                    chunk->offset_prev = next_offset;
                }

                return offset + sizeof(struct chunk);
            }
        }

        offset_prev = offset;
        offset = chunk->offset_next;
    }
}

uint32_t account_data_len(SolAccountInfo *ai, uint32_t offset)
{
    void *data = ai->data;

    // Nothing to do
    if (!offset)
        return 0;

    offset -= sizeof(struct chunk);

    struct chunk *chunk = data + offset;

    return chunk->length;
}

void account_data_free(SolAccountInfo *ai, uint32_t offset)
{
    void *data = ai->data;

    // Nothing to do
    if (!offset)
        return;

    offset -= sizeof(struct chunk);

    struct chunk *chunk = data + offset;

    chunk->allocated = false;

    // merge with previous chunk?
    if (chunk->offset_prev)
    {
        struct chunk *prev = data + chunk->offset_prev;

        if (!prev->allocated)
        {
            // merge
            offset = chunk->offset_prev;

            if (chunk->offset_next)
            {
                prev->length = chunk->offset_next - offset - sizeof(struct chunk);
                prev->offset_next = chunk->offset_next;

                struct chunk *next = data + chunk->offset_next;

                next->offset_prev = offset;
            }
            else
            {
                prev->offset_next = 0;
                prev->length = 0;
            }

            chunk = prev;
        }
    }

    // merge with next chunk?
    if (chunk->offset_next)
    {
        struct chunk *next = data + chunk->offset_next;

        if (!next->allocated)
        {
            // merge
            if (next->offset_next)
            {
                chunk->offset_next = next->offset_next;

                chunk->length = chunk->offset_next - offset - sizeof(struct chunk);

                struct chunk *next = data + chunk->offset_next;

                next->offset_prev = offset;
            }
            else
            {
                chunk->offset_next = 0;
                chunk->length = 0;
            }
        }
    }
}

#ifdef TEST
// To run the test:
// clang -DTEST -DSOL_TEST -O3 -Wall solana.c -o test && ./test
#include <assert.h>

void validate_heap(void *data, uint32_t offs[100])
{
    uint32_t offset = ((uint32_t *)data)[1];

    uint32_t last_offset = 0;

    for (;;)
    {
        struct chunk *chk = data + offset;

        //printf("chunk: offset:%x prev:%x next:%x length:%x allocated:%d\n", offset, chk->offset_prev, chk->offset_next, chk->length, chk->allocated);
        if (chk->length == 0 || chk->offset_next == 0)
        {
            assert(chk->length == 0 && chk->offset_next == 0 && chk->offset_prev == last_offset);
            //printf("last object at 0x%08x\n", offset);
            return;
        }

        assert(chk->offset_prev == last_offset && chk->length != 0);

        //printf("found object length %x at 0x%08lx allocated %d\n", chk->length, offset + sizeof(struct chunk), chk->allocated);

        assert(chk->offset_next - offset - sizeof(struct chunk) >= chk->length);

        if (chk->allocated)
        {
            bool found = false;
            uint32_t off = offset + sizeof(struct chunk);
            for (int i = 0; i < 100; i++)
            {
                if (offs[i] == off)
                {
                    assert(!found);
                    found = true;

                    uint8_t *mem = data + off;

                    for (int x = 0; x < 100; x++)
                    {
                        assert(mem[x] == i);
                    }
                }
            }
            assert(found);
        }
        else
        {
            // make sure we do not have this in our allocated list
            uint32_t off = offset + sizeof(struct chunk);
            for (int i = 0; i < 100; i++)
            {
                assert(offs[i] != off);
            }
        }

        last_offset = offset;
        offset = chk->offset_next;
    }
}

int main()
{
    uint8_t data[0x10000];
    SolAccountInfo ai;
    ai.data = data;
    ai.data_len = sizeof(data);
    uint32_t offs[100];
    uint32_t allocs = 0;

    memset(data, 0, sizeof(data));
    ((uint32_t *)data)[0] = 0x41424344;
    ((uint32_t *)data)[1] = 0x20;

    memset(offs, 0, sizeof(offs));

    int seed = time(NULL);
    printf("seed: %d\n", seed);
    srand(seed);

    for (;;)
    {
        validate_heap(data, offs);

        int n = rand() % 100;
        if (offs[n] == 0)
        {
            //printf("STEP: alloc %d\n", n);
            offs[n] = account_data_alloc(&ai, 100);
            memset(data + offs[n], n, 100);
        }
        else
        {
            //printf("STEP: free %d (0x%x)\n", n, offs[n]);
            account_data_free(&ai, offs[n]);
            offs[n] = 0;
        }
    }
}

void sol_panic_(const char *s, uint64_t len, uint64_t line, uint64_t column)
{
    printf("panic: %s line %lld", s, line);
}

void *sol_alloc_free_(uint64_t size, void *ptr)
{
    if (size)
    {
        return realloc(ptr, size);
    }
    else
    {
        free(ptr);
        return NULL;
    }
}

int solang_dispatch(const uint8_t *input, uint64_t input_len, SolAccountInfo *ka) {}
#endif
