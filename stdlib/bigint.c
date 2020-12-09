
#include <stdint.h>
#include <stdbool.h>

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

typedef unsigned _ExtInt(256) uint256_t;
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

typedef unsigned _ExtInt(512) uint512_t;
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
