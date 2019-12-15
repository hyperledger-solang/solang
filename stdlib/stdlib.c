// clang --target=wasm32 -c -emit-llvm -O3 -ffreestanding -fno-builtin -Wall stdlib.c
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>


/*
 * The external interface
 */

/*
 * Retrieve contract storage for this account. If nothing is stored at the key,
 * set the memory at dest to 0. If the storage is shorter, pad the remaining bytes
 * with 0.
 */
extern void get_storage32(uint32_t key, void *dest, int32_t length);
extern void set_storage32(uint32_t key, void *src, int32_t length);


/*
 */
__attribute__((visibility("hidden")))
void __memset8(void *_dest, uint64_t val, size_t length)
{
	uint64_t *dest = _dest;

	do {
		*dest++ = val;
	} while (--length);
}

/*
 * Our memcpy can only deal with multiples of 8 bytes. This is enough for
 * simple allocator below.
 */
__attribute__((visibility("hidden")))
void __memcpy8(void *_dest, void *_src, size_t length)
{
	uint64_t *dest = _dest;
	uint64_t *src = _src;

	do {
		*dest++ = *src++;
	} while (--length);
}

/*
 * Fast-ish clear, 8 bytes at a time.
 */
__attribute__((visibility("hidden")))
void __bzero8(void *_dest, size_t length)
{
	uint64_t *dest = _dest;

	do
		*dest++ = 0;
	while (--length);
}

/*
 * Fast-ish set, 8 bytes at a time.
 */
__attribute__((visibility("hidden")))
void __bset8(void *_dest, size_t length)
{
	int64_t *dest = _dest;

	do
		*dest++ = -1;
	while (--length);
}

/*
  There are many tradeoffs in heaps. I think for Solidity, we want:
   - small code size to reduce code length
   - not many malloc objects
   - not much memory

  So I think we should avoid fragmentation by neighbour merging. The most
  costly is walking the doubly linked list looking for free space.
*/
struct chunk {
	struct chunk *next, *prev;
	size_t length;
	bool allocated;
};

__attribute__((visibility("hidden")))
void __init_heap()
{
	struct chunk *first = (struct chunk*)0x10000;
	first->next = first->prev = NULL;
	first->allocated = false;
	first->length = (size_t)
		(__builtin_wasm_memory_size(0) -
	         (size_t)first - sizeof(struct chunk));
}

__attribute__((visibility("hidden")))
void __attribute__((noinline)) __free(void *m)
{
	struct chunk *cur = m;
	cur--;
	if (m) {
		cur->allocated = false;
		struct chunk *next = cur->next;
		if (next && !next->allocated) {
			// merge with next
			if ((cur->next = next->next) != NULL)
				cur->next->prev = cur;
			cur->length += next->length + sizeof(struct chunk);
			next = cur->next;
		}

		struct chunk *prev = cur->prev;
		if (prev && !prev->allocated) {
			// merge with previous
			prev->next = next;
			next->prev = prev;
			prev->length += cur->length + sizeof(struct chunk);
		}
	}
}

__attribute__((visibility("hidden")))
static void shrink_chunk(struct chunk *cur, size_t size)
{
	// round up to nearest 8 bytes
	size = (size + 7) & ~7;

	if (cur->length - size >= (8 + sizeof(struct chunk))) {
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

__attribute__((visibility("hidden")))
void* __attribute__((noinline)) __malloc(size_t size)
{
	struct chunk *cur = (struct chunk*)0x10000;

	while (cur && (cur->allocated || size > cur->length))
		cur = cur->next;

	if (cur) {
		shrink_chunk(cur, size);
		cur->allocated = true;
		return ++cur;
	} else {
		// go bang
		__builtin_unreachable();
	}
}

__attribute__((visibility("hidden")))
void* __realloc(void *m, size_t size)
{
	struct chunk *cur = m;

	cur--;

	struct chunk *next = cur->next;

	if (next && !next->allocated && size <=
		(cur->length + next->length + sizeof(struct chunk))) {
		// merge with next
		cur->next = next->next;
		cur->next->prev = cur;
		cur->length += next->length + sizeof(struct chunk);
		// resplit ..
		shrink_chunk(cur, size);
		return m;
	} else {
		void *n = __malloc(size);
		__memcpy8(n, m, size / 8);
		__free(m);
		return n;
	}
}

// This function is used for abi decoding integers.
// ABI encoding is big endian, and can have integers of 8 to 256 bits
// (1 to 32 bytes). This function copies length bytes and reverses the
// order since wasm is little endian.
__attribute__((visibility("hidden")))
void __be32toleN(uint8_t *from, uint8_t *to, uint32_t length)
{
	from += 31;

	do {
		*to++ = *from--;
	} while (--length);
}

// This function is for used for abi encoding integers
// ABI encoding is big endian.
__attribute__((visibility("hidden")))
void __leNtobe32(uint8_t *from, uint8_t *to, uint32_t length)
{
	to += 31;

	do {
		*to-- = *from++;
	} while (--length);
}

__attribute__((visibility("hidden")))
void __mul32(uint32_t left[], uint32_t right[], uint32_t out[], int len)
{
	uint64_t val1, val2, val3, carry = 0;
	int i;

	val1 = (uint64_t)left[0] * (uint64_t)right[0];
	carry = val1 >> 32;

	out[0] = val1;

	for (i = 0; i < (len - 1); i++) {
		val2 = (uint64_t)left[i] * (uint64_t)right[i + 1];
		val3 = (uint64_t)left[i + 1] * (uint64_t)right[i];
		bool overflow = __builtin_add_overflow(val2, val3, &val1);
		if (__builtin_add_overflow(val1, carry & 0xffffffff, &val1))
			overflow = true;
		out[i+1] = val1;
		carry >>= 32;
		carry += val1 >> 32;
		if (overflow)
			carry |= 0x100000000;
	}
}
