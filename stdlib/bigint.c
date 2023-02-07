// SPDX-License-Identifier: Apache-2.0

#include <stdint.h>
#include <stdbool.h>

/*
    In wasm/bpf, the instruction for multiplying two 64 bit values results in a 64 bit value. In
    other words, the result is truncated. The largest values we can multiply without truncation
    is 32 bit (by casting to 64 bit and doing a 64 bit multiplication). So, we divvy the work
    up into a 32 bit multiplications.

    No overflow checking is done.

    0		0		0		r5		r4		r3		r2		r1
    0		0		0		0		l4		l3		l2		l1 *
    ------------------------------------------------------------
    0		0		0		r5*l1	r4*l1	r3*l1	r2*l1	r1*l1
    0		0		r5*l2	r4*l2	r3*l2	r2*l2 	r1*l2	0
    0		r5*l3	r4*l3	r3*l3	r2*l3 	r1*l3	0		0
    r5*l4	r4*l4	r3*l4	r2*l4 	r1*l4	0		0 		0  +
    ------------------------------------------------------------
*/
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

// A version of __mul32 that detects overflow.
bool __mul32_with_builtin_ovf(uint32_t left[], uint32_t right[], uint32_t out[], int len)
{
    bool overflow = false;
    uint64_t val1 = 0, carry = 0;
    int left_len = len, right_len = len;
    while (left_len > 0 && !left[left_len - 1])
        left_len--;
    while (right_len > 0 && !right[right_len - 1])
        right_len--;
    int right_start = 0, right_end = 0;
    int left_start = 0;
    // We extend len to check for possible overflow. len = bit_width / 32. Checking for overflow for intN (where N = number of bits) requires checking for any set bits beyond N up to N*2.
    len = len * 2;
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

        // If the loop is within the operand bit size, just do the assignment
        if (l < len / 2)
        {
            out[l] = val1;
        }

        // If the loop extends to more than the bit size, we check for overflow.
        else if (l >= len / 2)
        {
            if (val1 > 0)
            {
                overflow = true;
                break;
            }
        }

        val1 = (val1 >> 32) | carry;
        carry = 0;
    }
    return overflow;
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

// Return the highest set bit in v
int bits(uint64_t v)
{
    int h = 63;

    if (!(v & 0xffffffff00000000))
    {
        h -= 32;
        v <<= 32;
    }

    if (!(v & 0xffff000000000000))
    {
        h -= 16;
        v <<= 16;
    }

    if (!(v & 0xff00000000000000))
    {
        h -= 8;
        v <<= 8;
    }

    if (!(v & 0xf000000000000000))
    {
        h -= 4;
        v <<= 4;
    }

    if (!(v & 0xc000000000000000))
    {
        h -= 2;
        v <<= 2;
    }

    if (!(v & 0x8000000000000000))
    {
        h -= 1;
    }

    return h;
}

int bits128(__uint128_t v)
{
    uint64_t upper = v >> 64;

    if (upper)
    {
        return bits(upper) + 64;
    }
    else
    {
        return bits(v);
    }
}

__uint128_t shl128(__uint128_t val, int r)
{
    if (r == 0)
    {
        return val;
    }
    else if (r & 64)
    {
        // Shift more than or equal 64
        uint64_t low = val;
        __uint128_t tmp = low << (r & 63);
        return tmp << 64;
    }
    else
    {
        // Shift less than 64
        uint64_t low = val;
        uint64_t high = val >> 64;

        __uint128_t tmp = (high << r) | (low >> (64 - r));

        return (low << r) | (tmp << 64);
    }
}

__uint128_t shr128(__uint128_t val, int r)
{
    if (r == 0)
    {
        return val;
    }
    else if (r & 64)
    {
        // Shift more than or equal 64
        uint64_t high = val >> 64;
        high >>= r & 63;

        return high;
    }
    else
    {
        // Shift less than 64
        uint64_t low = val;
        uint64_t high = val >> 64;

        low >>= r;
        high <<= 64 - r;

        __uint128_t tmp = high;

        return low | (tmp << 64);
    }
}

int udivmod128(__uint128_t *pdividend, __uint128_t *pdivisor, __uint128_t *remainder, __uint128_t *quotient)
{
    __uint128_t dividend = *pdividend;
    __uint128_t divisor = *pdivisor;

    if (divisor == 0)
        return 1;

    if (divisor == 1)
    {
        *remainder = 0;
        *quotient = dividend;
        return 0;
    }

    if (divisor == dividend)
    {
        *remainder = 0;
        *quotient = 1;
        return 0;
    }

    if (dividend == 0 || dividend < divisor)
    {
        *remainder = dividend;
        *quotient = 0;
        return 0;
    }

    __uint128_t q = 0, r = 0;

    for (int x = bits128(dividend) + 1; x > 0; x--)
    {
        q <<= 1;
        r <<= 1;

        if ((dividend >> (x - 1)) & 1)
        {
            r++;
        }

        if (r >= divisor)
        {
            r -= divisor;
            q++;
        }
    }

    *quotient = q;
    *remainder = r;

    return 0;
}

int sdivmod128(__uint128_t *pdividend, __uint128_t *pdivisor, __uint128_t *remainder, __uint128_t *quotient)
{
    bool dividend_negative = ((uint8_t *)pdividend)[15] >= 128;

    if (dividend_negative)
    {
        __uint128_t dividend = *pdividend;
        *pdividend = -dividend;
    }

    bool divisor_negative = ((uint8_t *)pdivisor)[15] >= 128;

    if (divisor_negative)
    {
        __uint128_t divisor = *pdivisor;
        *pdivisor = -divisor;
    }

    if (udivmod128(pdividend, pdivisor, remainder, quotient))
    {
        return 1;
    }

    if (dividend_negative != divisor_negative)
    {
        __uint128_t q = *quotient;
        *quotient = -q;
    }

    if (dividend_negative)
    {
        __uint128_t r = *remainder;
        *remainder = -r;
    }

    return 0;
}

typedef unsigned _BitInt(256) uint256_t;
uint256_t const uint256_0 = (uint256_t)0;
uint256_t const uint256_1 = (uint256_t)1;

int bits256(uint256_t *value)
{
    // 256 bits values consist of 4 uint64_ts.
    uint64_t *v = (uint64_t *)value;

    for (int i = 3; i >= 0; i--)
    {
        if (v[i])
            return bits(v[i]) + 64 * i;
    }

    return 0;
}

int udivmod256(uint256_t *pdividend, uint256_t *pdivisor, uint256_t *remainder, uint256_t *quotient)
{
    uint256_t dividend = *pdividend;
    uint256_t divisor = *pdivisor;

    if (divisor == uint256_0)
        return 1;

    if (divisor == uint256_1)
    {
        *remainder = uint256_0;
        *quotient = dividend;
        return 0;
    }

    if (divisor == dividend)
    {
        *remainder = uint256_0;
        *quotient = uint256_1;
        return 0;
    }

    if (dividend == uint256_0 || dividend < divisor)
    {
        *remainder = dividend;
        *quotient = uint256_0;
        return 0;
    }

    uint256_t q = uint256_0, r = dividend;

    uint256_t copyd = divisor << (bits256(&dividend) - bits256(&divisor));
    uint256_t adder = uint256_1 << (bits256(&dividend) - bits256(&divisor));

    if (copyd > dividend)
    {
        copyd >>= 1;
        adder >>= 1;
    }

    while (r >= divisor)
    {
        if (r >= copyd)
        {
            r -= copyd;
            q |= adder;
        }

        copyd >>= 1;
        adder >>= 1;
    }

    *quotient = q;
    *remainder = r;

    return 0;
}

int sdivmod256(uint256_t *pdividend, uint256_t *pdivisor, uint256_t *remainder, uint256_t *quotient)
{
    bool dividend_negative = ((uint8_t *)pdividend)[31] >= 128;

    if (dividend_negative)
    {
        uint256_t dividend = *pdividend;
        *pdividend = -dividend;
    }

    bool divisor_negative = ((uint8_t *)pdivisor)[31] >= 128;

    if (divisor_negative)
    {
        uint256_t divisor = *pdivisor;
        *pdivisor = -divisor;
    }

    if (udivmod256(pdividend, pdivisor, remainder, quotient))
    {
        return 1;
    }

    if (dividend_negative != divisor_negative)
    {
        uint256_t q = *quotient;
        *quotient = -q;
    }

    if (dividend_negative)
    {
        uint256_t r = *remainder;
        *remainder = -r;
    }

    return 0;
}

typedef unsigned _BitInt(512) uint512_t;
uint512_t const uint512_0 = (uint512_t)0;
uint512_t const uint512_1 = (uint512_t)1;

int bits512(uint512_t *value)
{
    // 512 bits values consist of 8 uint64_ts.
    uint64_t *v = (uint64_t *)value;

    for (int i = 7; i >= 0; i--)
    {
        if (v[i])
            return bits(v[i]) + 64 * i;
    }

    return 0;
}

int udivmod512(uint512_t *pdividend, uint512_t *pdivisor, uint512_t *remainder, uint512_t *quotient)
{
    uint512_t dividend = *pdividend;
    uint512_t divisor = *pdivisor;

    if (divisor == uint512_0)
        return 1;

    if (divisor == uint512_1)
    {
        *remainder = uint512_0;
        *quotient = dividend;
        return 0;
    }

    if (divisor == dividend)
    {
        *remainder = uint512_0;
        *quotient = uint512_1;
        return 0;
    }

    if (dividend == uint512_0 || dividend < divisor)
    {
        *remainder = dividend;
        *quotient = uint512_0;
        return 0;
    }

    uint512_t q = uint512_0, r = dividend;

    uint512_t copyd = divisor << (bits512(&dividend) - bits512(&divisor));
    uint512_t adder = uint512_1 << (bits512(&dividend) - bits512(&divisor));

    if (copyd > dividend)
    {
        copyd >>= 1;
        adder >>= 1;
    }

    while (r >= divisor)
    {
        if (r >= copyd)
        {
            r -= copyd;
            q |= adder;
        }

        copyd >>= 1;
        adder >>= 1;
    }

    *quotient = q;
    *remainder = r;

    return 0;
}

int sdivmod512(uint512_t *pdividend, uint512_t *pdivisor, uint512_t *remainder, uint512_t *quotient)
{
    bool dividend_negative = ((uint8_t *)pdividend)[63] >= 128;

    if (dividend_negative)
    {
        uint512_t dividend = *pdividend;
        *pdividend = -dividend;
    }

    bool divisor_negative = ((uint8_t *)pdivisor)[63] >= 128;

    if (divisor_negative)
    {
        uint512_t divisor = *pdivisor;
        *pdivisor = -divisor;
    }

    if (udivmod512(pdividend, pdivisor, remainder, quotient))
    {
        return 1;
    }

    if (dividend_negative != divisor_negative)
    {
        uint512_t q = *quotient;
        *quotient = -q;
    }

    if (dividend_negative)
    {
        uint512_t r = *remainder;
        *remainder = -r;
    }

    return 0;
}
