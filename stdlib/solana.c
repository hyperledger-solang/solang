
#include <stdint.h>
#include <stddef.h>
#include "stdlib.h"
#include "solana_sdk.h"

extern uint64_t solang_dispatch(const SolParameters *param);

// The address 'SysvarC1ock11111111111111111111111111111111' base58 decoded
static const SolPubkey clock_address = {0x06, 0xa7, 0xd5, 0x17, 0x18, 0xc7, 0x74, 0xc9, 0x28, 0x56, 0x63, 0x98, 0x69, 0x1d, 0x5e, 0xb6, 0x8b, 0x5e, 0xb8, 0xa3, 0x9b, 0x4b, 0x6d, 0x5c, 0x73, 0x55, 0x5b, 0x21, 0x00, 0x00, 0x00, 0x00};

uint64_t
entrypoint(const uint8_t *input)
{
    SolParameters params;

    uint64_t ret = sol_deserialize(input, &params);
    if (ret)
    {
        return ret;
    }

    int account_no;

    params.ka_clock = NULL;
    params.ka_cur = UINT64_MAX;

    for (account_no = 0; account_no < params.ka_num; account_no++)
    {
        const SolAccountInfo *acc = &params.ka[account_no];

        if (SolPubkey_same(params.account_id, acc->key))
        {
            params.ka_cur = account_no;
        }
        else if (SolPubkey_same(&clock_address, acc->key))
        {
            params.ka_clock = acc;
        }
    }

    if (params.ka_cur == UINT64_MAX)
    {
        return ERROR_INVALID_INSTRUCTION_DATA;
    }

    return solang_dispatch(&params);
}

void *__malloc(uint32_t size)
{
    return sol_alloc_free_(size, NULL);
}

uint64_t sol_invoke_signed_c(
    const SolInstruction *instruction,
    const SolAccountInfo *account_infos,
    int account_infos_len,
    const SolSignerSeeds *signers_seeds,
    int signers_seeds_len);

uint64_t external_call(uint8_t *input, uint32_t input_len, SolParameters *params)
{
    // The first 32 bytes of the input is the destination address
    const SolPubkey *dest = (const SolPubkey *)input;

    SolAccountMeta metas[10];
    SolInstruction instruction = {
        .program_id = NULL,
        .accounts = metas,
        .account_len = params->ka_num,
        .data = input,
        .data_len = input_len,
    };

    for (int account_no = 0; account_no < params->ka_num; account_no++)
    {
        const SolAccountInfo *acc = &params->ka[account_no];

        if (SolPubkey_same(dest, acc->key))
        {
            instruction.program_id = acc->owner;
            params->ka_last_called = acc;
        }

        metas[account_no].pubkey = acc->key;
        metas[account_no].is_writable = acc->is_writable;
        metas[account_no].is_signer = acc->is_signer;
    }

    if (instruction.program_id)
    {
        return sol_invoke_signed_c(&instruction, params->ka, params->ka_num, NULL, 0);
    }
    else
    {
        sol_log("call to account not in transaction");

        return ERROR_INVALID_ACCOUNT_DATA;
    }
}

// This function creates a new address and calls its constructor.
uint64_t create_contract(uint8_t *input, uint32_t input_len, uint64_t lamports, SolParameters *params)
{
    const SolAccountInfo *new_acc = NULL;

    SolAccountMeta metas[10];
    const SolInstruction instruction = {
        .program_id = (SolPubkey *)params->program_id,
        .accounts = metas,
        .account_len = params->ka_num,
        .data = input,
        .data_len = input_len,
    };

    // A fresh account must be provided by the caller; find it
    for (int account_no = 0; account_no < params->ka_num; account_no++)
    {
        const SolAccountInfo *acc = &params->ka[account_no];

        uint64_t *data = (uint64_t *)acc->data;

        if (!new_acc && !*data && SolPubkey_same(params->program_id, acc->owner))
        {
            new_acc = acc;
            params->ka_last_called = new_acc;
        }

        metas[account_no].pubkey = acc->key;
        metas[account_no].is_writable = acc->is_writable;
        metas[account_no].is_signer = acc->is_signer;
    }

    if (!new_acc)
    {
        sol_log("create contract requires a new account");

        return ERROR_NEW_ACCOUNT_NEEDED;
    }

    __memcpy8(input, new_acc->key->x, SIZE_PUBKEY / 8);

    return sol_invoke_signed_c(&instruction, params->ka, params->ka_num, NULL, 0);
}

struct clock_layout
{
    uint64_t slot;
    uint64_t epoch_start_timestamp;
    uint64_t epoch;
    uint64_t leader_schedule_epoch;
    uint64_t unix_timestamp;
};

uint64_t sol_timestamp(SolParameters *params)
{
    if (!params->ka_clock)
    {
        sol_log("clock account missing from transaction");
        sol_panic();
    }

    struct clock_layout *clock_data = (struct clock_layout *)params->ka_clock->data;

    return clock_data->unix_timestamp;
}

struct account_data_header
{
    uint32_t magic;
    uint32_t returndata_len;
    uint32_t returndata_offset;
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

uint64_t account_data_alloc(SolAccountInfo *ai, uint32_t size, uint32_t *res)
{
    void *data = ai->data;
    struct account_data_header *hdr = data;

    if (!size)
    {
        *res = 0;
        return 0;
    }

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

                if (offset + alloc_size + sizeof(struct chunk) >= ai->data_len)
                {
                    return ERROR_ACCOUNT_DATA_TOO_SMALL;
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

                *res = offset;
                return 0;
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

                *res = offset + sizeof(struct chunk);
                return 0;
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

                *res = offset + sizeof(struct chunk);
                return 0;
            }
        }

        offset_prev = offset;
        offset = chunk->offset_next;
    }
}

uint32_t account_data_len(void *data, uint32_t offset)
{
    // Nothing to do
    if (!offset)
        return 0;

    offset -= sizeof(struct chunk);

    struct chunk *chunk = data + offset;

    return chunk->length;
}

void account_data_free(void *data, uint32_t offset)
{
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

uint64_t account_data_realloc(SolAccountInfo *ai, uint32_t offset, uint32_t size, uint32_t *res)
{
    if (!size)
    {
        account_data_free(ai, offset);
        return 0;
    }

    if (!offset)
    {
        return account_data_alloc(ai, size, res);
    }

    void *data = ai->data;

    uint32_t chunk_offset = offset - sizeof(struct chunk);

    struct chunk *chunk = data + chunk_offset;
    struct chunk *next = data + chunk->offset_next;

    uint32_t existing_size = chunk->offset_next - offset;
    uint32_t alloc_size = ROUND_UP(size, 8);

    // 1. Is the existing chunk big enough
    if (size <= existing_size)
    {
        chunk->length = size;

        // can we free up some space
        if (existing_size >= alloc_size + sizeof(struct chunk) + 8)
        {
            uint32_t new_next_offset = offset + alloc_size;

            if (!next->allocated)
            {
                // merge with next chunk
                if (!next->offset_next)
                {
                    // the trailing free chunk
                    chunk->offset_next = new_next_offset;
                    next = data + new_next_offset;
                    next->offset_prev = chunk_offset;
                    next->offset_next = 0;
                    next->allocated = false;
                    next->length = 0;
                }
                else
                {
                    // merge with next chunk
                    chunk->offset_next = new_next_offset;
                    uint32_t offset_next_next = next->offset_next;

                    next = data + new_next_offset;
                    next->offset_prev = chunk_offset;
                    next->offset_next = offset_next_next;
                    next->allocated = false;
                    next->length = offset_next_next - new_next_offset - sizeof(struct chunk);

                    next = data + offset_next_next;
                    next->offset_prev = new_next_offset;
                }
            }
            else
            {
                // insert a new chunk
                uint32_t offset_next_next = chunk->offset_next;

                chunk->offset_next = new_next_offset;
                next = data + new_next_offset;
                next->offset_prev = chunk_offset;
                next->offset_next = offset_next_next;
                next->allocated = false;
                next->length = offset_next_next - new_next_offset - sizeof(struct chunk);

                next = data + offset_next_next;
                next->offset_prev = new_next_offset;
            }
        }

        *res = offset;
        return 0;
    }

    // 2. Can we use the next chunk to expand our chunk to fit
    // Note because we always merge neighbours, free chunks do not have free
    // neighbours.
    if (!next->allocated)
    {
        if (next->offset_next)
        {
            uint32_t merged_size = next->offset_next - offset;

            if (size < merged_size)
            {
                if (merged_size - alloc_size < 8 + sizeof(struct chunk))
                {
                    // merge the two chunks
                    chunk->offset_next = next->offset_next;
                    chunk->length = size;
                    next = data + chunk->offset_next;
                    next->offset_prev = chunk_offset;
                }
                else
                {
                    // expand our chunk to fit and shrink the next chunk
                    uint32_t offset_next = offset + alloc_size;
                    uint32_t offset_next_next = next->offset_next;

                    chunk->offset_next = offset_next;
                    chunk->length = size;

                    next = data + offset_next;
                    next->offset_prev = chunk_offset;
                    next->offset_next = offset_next_next;
                    next->length = offset_next_next - offset_next - sizeof(struct chunk);
                    next->allocated = false;

                    next = data + offset_next_next;
                    next->offset_prev = offset_next;
                }

                *res = offset;
                return 0;
            }
        }
        else
        {
            if (offset + alloc_size + sizeof(struct chunk) < ai->data_len)
            {
                chunk->offset_next = offset + alloc_size;
                chunk->length = size;

                next = data + chunk->offset_next;

                next->offset_prev = chunk_offset;
                next->offset_next = 0;
                next->allocated = false;
                next->length = 0;

                *res = offset;
                return 0;
            }
        }
    }

    uint32_t old_length = account_data_len(data, offset);
    uint32_t new_offset;
    uint64_t rc = account_data_alloc(ai, size, &new_offset);
    if (rc)
        return rc;

    __memcpy(data + new_offset, data + offset, old_length);
    account_data_free(ai, offset);

    *res = new_offset;
    return 0;
}

#ifdef TEST
// To run the test:
// clang -DTEST -DSOL_TEST -O3 -Wall solana.c stdlib.c -o test && ./test
#include <assert.h>

void validate_heap(void *data, uint32_t offs[100], uint32_t lens[100])
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

                    for (int x = 0; x < lens[i]; x++)
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
    uint32_t offs[100], lens[100];
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
        validate_heap(data, offs, lens);

        int n = rand() % 100;
        if (offs[n] == 0)
        {
            //printf("STEP: alloc %d\n", n);
            offs[n] = account_data_alloc(&ai, 100);
            memset(data + offs[n], n, 100);
            lens[n] = 100;
        }
        else if (rand() % 2)
        {
            //printf("STEP: free %d (0x%x)\n", n, offs[n]);
            account_data_free(&ai, offs[n]);
            offs[n] = 0;
        }
        else
        {
            int size = (rand() % 200) + 10;
            int old_size = account_data_len(&ai, offs[n]);
            offs[n] = account_data_realloc(&ai, offs[n], size);
            if (size > old_size)
                memset(data + offs[n] + old_size, n, size - old_size);
            lens[n] = size;
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
