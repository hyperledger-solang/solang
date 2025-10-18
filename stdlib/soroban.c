// SPDX-License-Identifier: Apache-2.0
// Minimal WASM bump allocator in C (no free).
// Exports:
//   soroban_alloc(size)                -> void*
//   soroban_alloc_align(size, align)   -> void*
//   soroban_alloc_init(size, init_ptr) -> struct vector*
//       Returns a pointer to a `struct vector` (see stdlib.h),
//       with `len` and `size` set to `size` and `data` initialized
//       from `init_ptr` if provided.
//   soroban_malloc(size)               -> void*
//   soroban_realloc(ptr, new_size)     -> void*   (COPY using header)
//   soroban_realloc_with_old(ptr, old_size, new_size) -> void* (explicit copy)
//   soroban_free(ptr, size, align)     -> void    (no-op)

#include <stdint.h>
#include <stddef.h>
#include "stdlib.h"

#ifndef SOROBAN_PAGE_LOG2
#define SOROBAN_PAGE_LOG2 16u // 64 KiB
#endif
#define SOROBAN_PAGE_SIZE (1u << SOROBAN_PAGE_LOG2)
#define SOROBAN_MEM_INDEX 0 // wasm memory #0

// clang/LLVM wasm32 intrinsics
static inline uint32_t wasm_memory_size_pages(void)
{
    return (uint32_t)__builtin_wasm_memory_size(SOROBAN_MEM_INDEX);
}
static inline int32_t wasm_memory_grow_pages(uint32_t delta_pages)
{
    return (int32_t)__builtin_wasm_memory_grow(SOROBAN_MEM_INDEX, (int)delta_pages);
}

static uint32_t g_cursor = 0; // current bump (bytes)
static uint32_t g_limit = 0;  // grown end (bytes)

// We prepend a small header before each returned allocation in order to
// remember the allocation size. This enables `realloc`-style copying even
// though this is a bump allocator without frees.
typedef struct
{
    uint32_t size; // payload size in bytes (not including this header)
} soroban_hdr_t;

static inline void *hdr_to_ptr(soroban_hdr_t *h)
{
    return (void *)((uintptr_t)h + sizeof(soroban_hdr_t));
}
static inline soroban_hdr_t *ptr_to_hdr(void *p)
{
    return (soroban_hdr_t *)((uintptr_t)p - sizeof(soroban_hdr_t));
}

static inline void *mem_copy(void *dst, const void *src, uint32_t n)
{
    // Simple, portable copy to avoid pulling in libc in freestanding mode
    unsigned char *d = (unsigned char *)dst;
    const unsigned char *s = (const unsigned char *)src;
    for (uint32_t i = 0; i < n; i++)
        d[i] = s[i];
    return dst;
}

static inline uint32_t align_up(uint32_t addr, uint32_t align)
{
    if (align == 0)
        align = 1;
    uint32_t mask = align - 1;
    return (addr + mask) & ~mask;
}

static inline void maybe_init(void)
{
    if (g_limit == 0)
    {
        uint32_t end = wasm_memory_size_pages() << SOROBAN_PAGE_LOG2; // bytes
        g_cursor = end;
        g_limit = end;
    }
}

// grow so that `need_bytes` fits (<== need_bytes is a byte address)
static inline int ensure_capacity(uint32_t need_bytes)
{
    if (need_bytes <= g_limit)
        return 1;
    uint32_t deficit = need_bytes - g_limit;
    uint32_t pages = (deficit + (SOROBAN_PAGE_SIZE - 1)) >> SOROBAN_PAGE_LOG2;
    if (wasm_memory_grow_pages(pages) < 0)
        return 0; // OOM
    g_limit += pages << SOROBAN_PAGE_LOG2;
    return 1;
}

static void *alloc_impl(uint32_t bytes, uint32_t align)
{
    maybe_init();
    // Ensure there is space for the header while keeping the returned pointer
    // aligned as requested.
    uint32_t start = align_up(g_cursor + (uint32_t)sizeof(soroban_hdr_t), align ? align : 1);
    uint32_t end = start + bytes;

    if (end > g_limit)
    {
        if (!ensure_capacity(end))
            return (void *)0; // OOM
        // retry after growth
        start = align_up(g_cursor + (uint32_t)sizeof(soroban_hdr_t), align ? align : 1);
        end = start + bytes;
    }
    g_cursor = end;
    // Write header just before the returned pointer
    soroban_hdr_t *hdr = (soroban_hdr_t *)(uintptr_t)(start - (uint32_t)sizeof(soroban_hdr_t));
    hdr->size = bytes;
    return (void *)(uintptr_t)start;
}

// -------------------- exported API --------------------

__attribute__((export_name("soroban_alloc"))) void *soroban_alloc(uint32_t size)
{
    // default alignment 8
    return alloc_impl(size, 8);
}

__attribute__((export_name("soroban_alloc_init"))) struct vector *soroban_alloc_init(uint32_t members,
                                                                                     const void *init_ptr)
{
    // Emulate stdlib.c:vector_new() but allocate via alloc_impl.
    // Note: here `members` is the number of bytes in the vector payload
    // (element size assumed to be 1 for Soroban at present).
    uint32_t size_array = members;

    struct vector *v = (struct vector *)alloc_impl((uint32_t)sizeof(struct vector) + size_array, 8);
    if (v == (struct vector *)0)
    {
        return (struct vector *)0;
    }

    v->len = members;
    v->size = members;

    uint8_t *data = v->data;

    if (size_array)
    {
        if (init_ptr != (const void *)0)
        {
            mem_copy(data, init_ptr, size_array);
        }
        else
        {
            // zero-initialize when no initializer provided
            for (uint32_t i = 0; i < size_array; i++)
                data[i] = 0;
        }
    }

    return v;
}

__attribute__((export_name("soroban_alloc_align"))) void *soroban_alloc_align(uint32_t size, uint32_t align)
{
    return alloc_impl(size, align);
}

__attribute__((export_name("soroban_malloc"))) void *soroban_malloc(uint32_t size)
{
    return alloc_impl(size, 8);
}

// Reallocate and copy previous contents. Since we store a small header in
// front of each allocation, we can determine the old size here and copy the
// minimum of old and new sizes.
__attribute__((export_name("soroban_realloc"))) void *soroban_realloc(void *old_ptr, uint32_t new_size)
{
    if (old_ptr == (void *)0)
    {
        return alloc_impl(new_size, 8);
    }

    // Determine old size from the header placed before the allocation
    soroban_hdr_t *old_hdr = ptr_to_hdr(old_ptr);
    uint32_t old_size = old_hdr->size;

    void *new_ptr = alloc_impl(new_size, 8);
    if (new_ptr == (void *)0)
        return (void *)0; // OOM

    uint32_t copy = old_size < new_size ? old_size : new_size;
    if (copy)
        mem_copy(new_ptr, old_ptr, copy);
    return new_ptr;
}

// Variant that accepts the old size explicitly. Useful when the caller
// already knows the previous allocation size and wants to avoid relying on
// the header (or for interop with older allocations).
__attribute__((export_name("soroban_realloc_with_old"))) void *soroban_realloc_with_old(void *old_ptr,
                                                                                        uint32_t old_size,
                                                                                        uint32_t new_size)
{
    if (old_ptr == (void *)0)
    {
        return alloc_impl(new_size, 8);
    }
    void *new_ptr = alloc_impl(new_size, 8);
    if (new_ptr == (void *)0)
        return (void *)0; // OOM
    uint32_t copy = old_size < new_size ? old_size : new_size;
    if (copy)
        mem_copy(new_ptr, old_ptr, copy);
    return new_ptr;
}

__attribute__((export_name("soroban_free"))) void soroban_free(void *_ptr, uint32_t _size, uint32_t _align)
{
    (void)_ptr;
    (void)_size;
    (void)_align; // bump allocator: no-op
}
