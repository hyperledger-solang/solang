
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
