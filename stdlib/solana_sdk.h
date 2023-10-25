// SPDX-License-Identifier: Apache-2.0

#pragma once

/**
 * Numeric types
 */
#ifndef __LP64__
#error LP64 data model required
#endif

/** Indicates the instruction was processed successfully */
#define SUCCESS 0

/**
 * Builtin program status values occupy the upper 32 bits of the program return
 * value.  Programs may define their own error values but they must be confined
 * to the lower 32 bits.
 */
#define TO_BUILTIN(error) ((uint64_t)(error) << 32)

/** Note: Not applicable to program written in C */
#define ERROR_CUSTOM_ZERO TO_BUILTIN(1)
/** The arguments provided to a program instruction where invalid */
#define ERROR_INVALID_ARGUMENT TO_BUILTIN(2)
/** An instruction's data contents was invalid */
#define ERROR_INVALID_INSTRUCTION_DATA TO_BUILTIN(3)
/** An account's data contents was invalid */
#define ERROR_INVALID_ACCOUNT_DATA TO_BUILTIN(4)
/** An account's data was too small */
#define ERROR_ACCOUNT_DATA_TOO_SMALL TO_BUILTIN(5)
/** An account's balance was too small to complete the instruction */
#define ERROR_INSUFFICIENT_FUNDS TO_BUILTIN(6)
/** The account did not have the expected program id */
#define ERROR_INCORRECT_PROGRAM_ID TO_BUILTIN(7)
/** A signature was required but not found */
#define ERROR_MISSING_REQUIRED_SIGNATURES TO_BUILTIN(8)
/** An initialize instruction was sent to an account that has already been initialized */
#define ERROR_ACCOUNT_ALREADY_INITIALIZED TO_BUILTIN(9)
/** An attempt to operate on an account that hasn't been initialized */
#define ERROR_UNINITIALIZED_ACCOUNT TO_BUILTIN(10)
/** The instruction expected additional account keys */
#define ERROR_NOT_ENOUGH_ACCOUNT_KEYS TO_BUILTIN(11)
/** Note: Not applicable to program written in C */
#define ERROR_ACCOUNT_BORROW_FAILED TO_BUILTIN(12)
/** The length of the seed is too long for address generation */
#define MAX_SEED_LENGTH_EXCEEDED TO_BUILTIN(13)
/** Provided seeds do not result in a valid address */
#define INVALID_SEEDS TO_BUILTIN(14)
/** Need more account */
#define ERROR_NEW_ACCOUNT_NEEDED TO_BUILTIN(15)

/**
 * Boolean type
 */
#ifndef __cplusplus
#include <stdbool.h>
#endif

/**
 * Prints a string to stdout
 */
void sol_log_(const char *, uint64_t);
#define sol_log(message) sol_log_(message, sol_strlen(message))

/**
 * Prints a 64 bit values represented in hexadecimal to stdout
 */
void sol_log_64_(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
#define sol_log_64 sol_log_64_

/**
 * Size of Public key in bytes
 */
#define SIZE_PUBKEY 32

/**
 * Public key
 */
typedef struct
{
    uint8_t x[SIZE_PUBKEY];
} SolPubkey;

/**
 * Compares two public keys
 *
 * @param one First public key
 * @param two Second public key
 * @return true if the same
 */
static bool SolPubkey_same(const SolPubkey *one, const SolPubkey *two)
{
    for (int i = 0; i < sizeof(*one); i++)
    {
        if (one->x[i] != two->x[i])
        {
            return false;
        }
    }
    return true;
}

/**
 * Keyed Account
 */
typedef struct
{
    SolPubkey *key;      /** Public key of the account */
    uint64_t *lamports;  /** Number of lamports owned by this account */
    uint64_t data_len;   /** Length of data in bytes */
    uint8_t *data;       /** On-chain data within this account */
    SolPubkey *owner;    /** Program that owns this account */
    uint64_t rent_epoch; /** The epoch at which this account will next owe rent */
    bool is_signer;      /** Transaction was signed by this account's key? */
    bool is_writable;    /** Is the account writable? */
    bool executable;     /** This account's data contains a loaded program (and is now read-only) */
} SolAccountInfo;

/**
 * Copies memory
 */
static void sol_memcpy(void *dst, const void *src, int len)
{
    for (int i = 0; i < len; i++)
    {
        *((uint8_t *)dst + i) = *((const uint8_t *)src + i);
    }
}

/**
 * Compares memory
 */
static int sol_memcmp(const void *s1, const void *s2, int n)
{
    for (int i = 0; i < n; i++)
    {
        uint8_t diff = *((uint8_t *)s1 + i) - *((const uint8_t *)s2 + i);
        if (diff)
        {
            return diff;
        }
    }
    return 0;
}

/**
 * Fill a byte string with a byte value
 */
static void sol_memset(void *b, int c, size_t len)
{
    uint8_t *a = (uint8_t *)b;
    while (len > 0)
    {
        *a = c;
        a++;
        len--;
    }
}

/**
 * Find length of string
 */
static size_t sol_strlen(const char *s)
{
    size_t len = 0;
    while (*s)
    {
        len++;
        s++;
    }
    return len;
}

/**
 * Computes the number of elements in an array
 */
#define SOL_ARRAY_SIZE(a) (sizeof(a) / sizeof(a[0]))

/**
 * Panics
 *
 * Prints the line number where the panic occurred and then causes
 * the BPF VM to immediately halt execution. No accounts' data are updated
 */
void sol_panic_(const char *, uint64_t, uint64_t, uint64_t);
#define sol_panic() sol_panic_(__FILE__, sizeof(__FILE__), __LINE__, 0)

/**
 * Asserts
 */
#define sol_assert(expr)                                                                                               \
    if (!(expr))                                                                                                       \
    {                                                                                                                  \
        sol_panic();                                                                                                   \
    }

/**
 * Seed used to create a program address or passed to sol_invoke_signed
 */
typedef struct
{
    const uint8_t *addr; /** Seed bytes */
    uint64_t len;        /** Length of the seed bytes */
} SolSignerSeed;

/**
 * Structure that the program's entrypoint input data is deserialized into.
 */
typedef struct
{
    SolAccountInfo ka[10]; /** Pointer to an array of SolAccountInfo, must already
                          point to an array of SolAccountInfos */
    uint64_t ka_num;       /** Number of SolAccountInfo entries in `ka` */
    const uint8_t *input;  /** pointer to the instruction data */
    uint64_t input_len;    /** Length in bytes of the instruction data */
    SolPubkey *program_id; /** program_id of the currently executing program */
    const SolAccountInfo *ka_clock;
    const SolAccountInfo *ka_instructions;
} SolParameters;

/**
 * Maximum number of bytes a program may add to an account during a single realloc
 */
#define MAX_PERMITTED_DATA_INCREASE (1024 * 10)

/**
 * De-serializes the input parameters into usable types
 *
 * Use this function to deserialize the buffer passed to the program entrypoint
 * into usable types.  This function does not perform copy deserialization,
 * instead it populates the pointers and lengths in SolAccountInfo and data so
 * that any modification to lamports or account data take place on the original
 * buffer.  Doing so also eliminates the need to serialize back into the buffer
 * at the end of the program.
 *
 * @param input Source buffer containing serialized input parameters
 * @param params Pointer to a SolParameters structure
 * @return Boolean true if successful.
 */
static uint64_t sol_deserialize(const uint8_t *input, SolParameters *params)
{
    if (NULL == input || NULL == params)
    {
        return ERROR_INVALID_ARGUMENT;
    }

    uint64_t max_accounts = SOL_ARRAY_SIZE(params->ka);
    params->ka_num = *(uint64_t *)input;
    input += sizeof(uint64_t);

    for (int i = 0; i < params->ka_num; i++)
    {
        uint8_t dup_info = input[0];
        input += sizeof(uint8_t);

        if (i >= max_accounts)
        {
            if (dup_info == UINT8_MAX)
            {
                input += sizeof(uint8_t);
                input += sizeof(uint8_t);
                input += sizeof(uint8_t);
                input += 4; // padding
                input += sizeof(SolPubkey);
                input += sizeof(SolPubkey);
                input += sizeof(uint64_t);
                uint64_t data_len = *(uint64_t *)input;
                input += sizeof(uint64_t);
                input += data_len;
                input += MAX_PERMITTED_DATA_INCREASE;
                input = (uint8_t *)(((uint64_t)input + 8 - 1) & ~(8 - 1)); // padding
                input += sizeof(uint64_t);
            }
            else
            {
                input += 7; // padding for the 64-bit alignment
            }
            continue;
        }
        if (dup_info == UINT8_MAX)
        {
            // is signer?
            params->ka[i].is_signer = *(uint8_t *)input != 0;
            input += sizeof(uint8_t);

            // is writable?
            params->ka[i].is_writable = *(uint8_t *)input != 0;
            input += sizeof(uint8_t);

            // executable?
            params->ka[i].executable = *(uint8_t *)input;
            input += sizeof(uint8_t);

            input += 4; // padding

            // key
            params->ka[i].key = (SolPubkey *)input;
            input += sizeof(SolPubkey);

            // owner
            params->ka[i].owner = (SolPubkey *)input;
            input += sizeof(SolPubkey);

            // lamports
            params->ka[i].lamports = (uint64_t *)input;
            input += sizeof(uint64_t);

            // account data
            params->ka[i].data_len = *(uint64_t *)input;
            input += sizeof(uint64_t);
            params->ka[i].data = (uint8_t *)input;
            input += params->ka[i].data_len;
            input += MAX_PERMITTED_DATA_INCREASE;
            input = (uint8_t *)(((uint64_t)input + 8 - 1) & ~(8 - 1)); // padding

            // rent epoch
            params->ka[i].rent_epoch = *(uint64_t *)input;
            input += sizeof(uint64_t);
        }
        else
        {
            params->ka[i].is_signer = params->ka[dup_info].is_signer;
            params->ka[i].is_writable = params->ka[dup_info].is_writable;
            params->ka[i].executable = params->ka[dup_info].executable;
            params->ka[i].key = params->ka[dup_info].key;
            params->ka[i].owner = params->ka[dup_info].owner;
            params->ka[i].lamports = params->ka[dup_info].lamports;
            params->ka[i].data_len = params->ka[dup_info].data_len;
            params->ka[i].data = params->ka[dup_info].data;
            params->ka[i].rent_epoch = params->ka[dup_info].rent_epoch;
            input += 7; // padding
        }
    }

    uint64_t data_len = *(uint64_t *)input;
    input += sizeof(uint64_t);

    params->input_len = data_len;
    params->input = input;
    input += data_len;

    params->program_id = (SolPubkey *)input;
    input += sizeof(SolPubkey);

    if (params->ka_num > max_accounts)
        params->ka_num = max_accounts;

    return 0;
}

/**
 * Byte array pointer and string
 */
typedef struct
{
    const uint8_t *addr; /** bytes */
    uint64_t len;        /** number of bytes*/
} SolBytes;

/**
 * Length of a sha256 hash result
 */
#define SHA256_RESULT_LENGTH 32

/**
 * Sha256
 *
 * @param bytes Array of byte arrays
 * @param bytes_len Number of byte arrays
 * @param result 32 byte array to hold the result
 */
static uint64_t sol_sha256(const SolBytes *bytes, int bytes_len, const uint8_t *result);

/**
 * Account Meta
 */
typedef struct
{
    SolPubkey *pubkey; /** An account's public key */
    bool is_writable;  /** True if the `pubkey` can be loaded as a read-write account */
    bool is_signer;    /** True if an Instruction requires a Transaction signature matching `pubkey` */
} SolAccountMeta;

/**
 * Instruction
 */
typedef struct
{
    SolPubkey *program_id;    /** Pubkey of the instruction processor that executes this instruction */
    SolAccountMeta *accounts; /** Metadata for what accounts should be passed to the instruction processor */
    uint64_t account_len;     /** Number of SolAccountMetas */
    uint8_t *data;            /** Opaque data passed to the instruction processor */
    uint64_t data_len;        /** Length of the data in bytes */
} SolInstruction;

/**
 * Seeds used by a signer to create a program address or passed to
 * sol_invoke_signed
 */
typedef struct
{
    const SolSignerSeed *addr; /** An array of a signer's seeds */
    uint64_t len;              /** Number of seeds */
} SolSignerSeeds;

/**
 * Create a program address
 *
 * @param seeds Seed bytes used to sign program accounts
 * @param seeds_len Length of the seeds array
 * @param Progam id of the signer
 * @param Program address created, filled on return
 */
static uint64_t sol_create_program_address(const SolSignerSeed *seeds, int seeds_len, const SolPubkey *program_id,
                                           const SolPubkey *address);

/**
 * Cross-program invocation
 *  * @{
 */

/**
 * Invoke another program and sign for some of the keys
 *
 * @param instruction Instruction to process
 * @param account_infos Accounts used by instruction
 * @param account_infos_len Length of account_infos array
 * @param seeds Seed bytes used to sign program accounts
 * @param seeds_len Length of the seeds array
 */
static uint64_t sol_invoke_signed(const SolInstruction *instruction, const SolAccountInfo *account_infos,
                                  int account_infos_len, const SolSignerSeeds *signers_seeds, int signers_seeds_len)
{
    uint64_t sol_invoke_signed_c(const SolInstruction *instruction, const SolAccountInfo *account_infos,
                                 int account_infos_len, const SolSignerSeeds *signers_seeds, int signers_seeds_len);

    return sol_invoke_signed_c(instruction, account_infos, account_infos_len, signers_seeds, signers_seeds_len);
}
/**
 * Invoke another program
 *
 * @param instruction Instruction to process
 * @param account_infos Accounts used by instruction
 * @param account_infos_len Length of account_infos array
 */
static uint64_t sol_invoke(const SolInstruction *instruction, const SolAccountInfo *account_infos,
                           int account_infos_len)
{
    const SolSignerSeeds signers_seeds[] = {{}};
    return sol_invoke_signed(instruction, account_infos, account_infos_len, signers_seeds, 0);
}

/**@}*/

/**
 * Debugging utilities
 * @{
 */

/**
 * Prints the hexadecimal representation of a public key
 *
 * @param key The public key to print
 */
void sol_log_pubkey(const SolPubkey *pubkey);

/**
 * Prints the hexadecimal representation of an array
 *
 * @param array The array to print
 */
static void sol_log_array(const uint8_t *array, int len)
{
    for (int j = 0; j < len; j++)
    {
        sol_log_64(0, 0, 0, j, array[j]);
    }
}

/**
 * Prints the program's input parameters
 *
 * @param params Pointer to a SolParameters structure
 */
static void sol_log_params(const SolParameters *params)
{
    sol_log("- Program identifier:");
    sol_log_pubkey(params->program_id);

    sol_log("- Number of KeyedAccounts");
    sol_log_64(0, 0, 0, 0, params->ka_num);
    for (int i = 0; i < params->ka_num; i++)
    {
        sol_log("  - Is signer");
        sol_log_64(0, 0, 0, 0, params->ka[i].is_signer);
        sol_log("  - Is writable");
        sol_log_64(0, 0, 0, 0, params->ka[i].is_writable);
        sol_log("  - Key");
        sol_log_pubkey(params->ka[i].key);
        sol_log("  - Lamports");
        sol_log_64(0, 0, 0, 0, *params->ka[i].lamports);
        sol_log("  - data");
        sol_log_array(params->ka[i].data, params->ka[i].data_len);
        sol_log("  - Owner");
        sol_log_pubkey(params->ka[i].owner);
        sol_log("  - Executable");
        sol_log_64(0, 0, 0, 0, params->ka[i].executable);
        sol_log("  - Rent Epoch");
        sol_log_64(0, 0, 0, 0, params->ka[i].rent_epoch);
    }
    sol_log("- Eth abi Instruction data\0");
    sol_log_array(params->input, params->input_len);
}

/**@}*/

/**
 * Program instruction entrypoint
 *
 * @param input Buffer of serialized input parameters.  Use sol_deserialize() to decode
 * @return 0 if the instruction executed successfully
 */
uint64_t entrypoint(const uint8_t *input);

#ifdef SOL_TEST
/**
 * Stub log functions when building tests
 */
#include <stdio.h>
void sol_log_(const char *s, uint64_t len)
{
    printf("sol_log: %s\n", s);
}
void sol_log_64(uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5)
{
    printf("sol_log_64: %llu, %llu, %llu, %llu, %llu\n", arg1, arg2, arg3, arg4, arg5);
}
#endif

#ifdef __cplusplus
}
#endif

/**@}*/
