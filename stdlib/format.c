// SPDX-License-Identifier: Apache-2.0

#include <stdint.h>

void hex_encode(char *output, uint8_t *input, uint32_t length)
{
    for (int i = 0; i < length; i++)
    {
        uint8_t h = (input[i] >> 4);
        *output++ = h > 9 ? h + 'a' - 10 : '0' + h;
        uint8_t l = (input[i] & 0x0f);
        *output++ = l > 9 ? l + 'a' - 10 : '0' + l;
    }
}

void hex_encode_rev(char *output, uint8_t *input, uint32_t length)
{
    for (int i = length - 1; i >= 0; i--)
    {
        uint8_t h = (input[i] >> 4);
        *output++ = h > 9 ? h + 'a' - 10 : '0' + h;
        uint8_t l = (input[i] & 0x0f);
        *output++ = l > 9 ? l + 'a' - 10 : '0' + l;
    }
}

char *uint2hex(char *output, uint8_t *input, uint32_t length)
{
    // first count how many characters
    while (length > 1 && input[length - 1] == 0)
        length--;

    *output++ = '0';
    *output++ = 'x';

    uint8_t h = (input[length - 1] >> 4);
    if (h > 0)
        *output++ = h > 9 ? h + 'a' - 10 : '0' + h;
    uint8_t l = (input[length - 1] & 0x0f);
    *output++ = l > 9 ? l + 'a' - 10 : '0' + l;

    while (--length)
    {
        uint8_t h = (input[length - 1] >> 4);
        *output++ = h > 9 ? h + 'a' - 10 : '0' + h;
        uint8_t l = (input[length - 1] & 0x0f);
        *output++ = l > 9 ? l + 'a' - 10 : '0' + l;
    }

    return output;
}

char *uint2bin(char *output, uint8_t *input, uint32_t length)
{
    // first count how many bytes
    while (length > 1 && input[length - 1] == 0)
        length--;

    *output++ = '0';
    *output++ = 'b';

    uint8_t v = input[length - 1];

    int i = 8;

    while (i > 0 && !(v & 0x80))
    {
        v <<= 1;
        i--;
    }

    while (i--)
    {
        *output++ = v & 0x80 ? '1' : '0';
        v <<= 1;
    }

    while (--length)
    {
        uint8_t v = input[length - 1];
        for (i = 0; i < 8; i++)
        {
            *output++ = v & 0x80 ? '1' : '0';
            v <<= 1;
        }
    }

    return output;
}

char *uint2dec(char *output, uint64_t val)
{
    char buf[20];
    int len = 0;

    // first generate the digits in left-to-right
    do
    {
        buf[len++] = val % 10;
        val /= 10;
    } while (val);

    // now copy them in to right-to-left
    while (len--)
    {
        *output++ = buf[len] + '0';
    }

    return output;
}

extern int udivmod128(const __uint128_t *dividend, const __uint128_t *divisor, __uint128_t *remainder, __uint128_t *quotient);

char *uint128dec(char *output, __uint128_t val128)
{
    // we want 1e19, how to declare such a constant in clang?
    const __uint128_t billion = 10000000000;
    const __uint128_t divisor = billion * 1000000000;
    __uint128_t q, r;
    char buf[40];
    int len = 0;

    // first do the first 19 digits

    // divisor is never zero so we can ignore return value
    udivmod128(&val128, &divisor, &r, &q);

    uint64_t val = r;

    do
    {
        buf[len++] = val % 10;
        val /= 10;
    } while (val);

    /// next 19 digits
    udivmod128(&q, &divisor, &r, &q);

    val = r;

    if (val)
    {
        // add 0s
        while (len < 19)
        {
            buf[len++] = 0;
        }

        do
        {
            buf[len++] = val % 10;
            val /= 10;
        } while (val);
    }

    val = q;

    if (val)
    {
        // add 0s
        while (len < 38)
        {
            buf[len++] = 0;
        }

        do
        {
            buf[len++] = val % 10;
            val /= 10;
        } while (val);
    }

    // now copy them in to right-to-left
    while (len--)
    {
        *output++ = buf[len] + '0';
    }

    return output;
}

typedef unsigned _BitInt(256) uint256_t;

extern int udivmod256(const uint256_t *dividend, const uint256_t *divisor, uint256_t *remainder, uint256_t *quotient);

char *
uint256dec(char *output, uint256_t *val256)
{
    // we want 1e19, how to declare such a constant in clang?
    const uint256_t n1e10 = 10000000000;
    const uint256_t n1e9 = 1000000000;
    uint256_t divisor = n1e10 * n1e9;
    uint256_t q = *val256, r;
    char buf[80];
    int len = 0;

    // do the first digits
    for (int digits = 0; digits < 76; digits += 19)
    {
        // divisor is never zero so we can ignore return value
        udivmod256(&q, &divisor, &r, &q);

        uint64_t val = r;

        // add 0s
        while (len < digits)
        {
            buf[len++] = 0;
        }

        do
        {
            buf[len++] = val % 10;
            val /= 10;
        } while (val);

        if (q == (uint256_t)0)
        {
            break;
        }
    }

    uint64_t val = q;

    if (val)
    {
        do
        {
            buf[len++] = val % 10;
            val /= 10;
        } while (val);
    }

    // now copy them in to right-to-left
    while (len--)
    {
        *output++ = buf[len] + '0';
    }

    return output;
}
