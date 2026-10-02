// SPDX-License-Identifier: Apache-2.0

#include <stdint.h>

// r55 sets up no stack, and places the calldata at 0x80000000 as an 8 byte
// length followed by the payload.
asm(".section .text.start\n"
    ".globl _start\n"
    "_start:\n"
    "  la sp, _stack_top\n"
    // t1 = 0x80000000; `lui` would sign-extend it on RV64.
    "  li t1, 1\n"
    "  slli t1, t1, 31\n"
    "  lw a1, 0(t1)\n"   // calldata length
    "  addi a0, t1, 8\n" // calldata payload
    "  call solang_dispatch\n"
    // solang_dispatch ends with Return or Revert, so this is a bug. Revert:
    // returning would deploy an account with no code.
    "  li a0, 0\n"
    "  li a1, 0\n"
    "  li t0, 0xFD\n"
    "  ecall\n");

void __sys_return(const void *data, uint64_t len)
{
    register uint64_t a0 asm("a0") = (uint64_t)data;
    register uint64_t a1 asm("a1") = len;
    register uint64_t t0 asm("t0") = 0xF3; // Return
    asm volatile("ecall" : : "r"(a0), "r"(a1), "r"(t0) : "memory");
}

void __sys_revert(const void *data, uint64_t len)
{
    register uint64_t a0 asm("a0") = (uint64_t)data;
    register uint64_t a1 asm("a1") = len;
    register uint64_t t0 asm("t0") = 0xFD; // Revert
    asm volatile("ecall" : : "r"(a0), "r"(a1), "r"(t0) : "memory");
}

void __sys_sstore(uint64_t k0, uint64_t k1, uint64_t k2, uint64_t k3, uint64_t v0, uint64_t v1, uint64_t v2,
                  uint64_t v3)
{
    register uint64_t a0 asm("a0") = k0;
    register uint64_t a1 asm("a1") = k1;
    register uint64_t a2 asm("a2") = k2;
    register uint64_t a3 asm("a3") = k3;
    register uint64_t a4 asm("a4") = v0;
    register uint64_t a5 asm("a5") = v1;
    register uint64_t a6 asm("a6") = v2;
    register uint64_t a7 asm("a7") = v3;
    register uint64_t t0 asm("t0") = 0x55; // SStore
    asm volatile("ecall"
                 :
                 : "r"(a0), "r"(a1), "r"(a2), "r"(a3), "r"(a4), "r"(a5), "r"(a6), "r"(a7), "r"(t0)
                 : "memory");
}

// The result goes through `out`: returning a 32-byte struct would pass a
// hidden pointer in a0, which holds the key.
void __sys_sload(uint64_t k0, uint64_t k1, uint64_t k2, uint64_t k3, uint64_t *out)
{
    register uint64_t a0 asm("a0") = k0;
    register uint64_t a1 asm("a1") = k1;
    register uint64_t a2 asm("a2") = k2;
    register uint64_t a3 asm("a3") = k3;
    register uint64_t t0 asm("t0") = 0x54; // SLoad
    asm volatile("ecall" : "+r"(a0), "+r"(a1), "+r"(a2), "+r"(a3) : "r"(t0) : "memory");
    out[0] = a0;
    out[1] = a1;
    out[2] = a2;
    out[3] = a3;
}

// Writes the 20 byte address, big-endian.
void __sys_caller(uint8_t *out)
{
    register uint64_t a0 asm("a0");
    register uint64_t a1 asm("a1");
    register uint64_t a2 asm("a2");
    register uint64_t t0 asm("t0") = 0x33; // Caller
    asm volatile("ecall" : "=r"(a0), "=r"(a1), "=r"(a2) : "r"(t0) : "memory");
    for (int i = 0; i < 8; i++)
        out[i] = (a0 >> (56 - i * 8)) & 0xFF;
    for (int i = 0; i < 8; i++)
        out[8 + i] = (a1 >> (56 - i * 8)) & 0xFF;
    for (int i = 0; i < 4; i++)
        out[16 + i] = (a2 >> (56 - i * 8)) & 0xFF;
}

// Writes the 32 byte value, big-endian.
void __sys_callvalue(uint8_t *out)
{
    register uint64_t a0 asm("a0");
    register uint64_t a1 asm("a1");
    register uint64_t a2 asm("a2");
    register uint64_t a3 asm("a3");
    register uint64_t t0 asm("t0") = 0x34; // CallValue
    asm volatile("ecall" : "=r"(a0), "=r"(a1), "=r"(a2), "=r"(a3) : "r"(t0) : "memory");

    uint64_t parts[4] = {a0, a1, a2, a3};
    for (int i = 0; i < 4; i++)
    {
        for (int j = 0; j < 8; j++)
        {
            out[i * 8 + j] = (parts[i] >> (56 - j * 8)) & 0xFF;
        }
    }
}
