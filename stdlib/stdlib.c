// clang --target=wasm32 -c -emit-llvm -O3 -ffreestanding -fno-builtin -Wall stdlib.c
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#include "stdlib.h"

/*
 */
void __memset8(void *_dest, uint64_t val, size_t length)
{
	uint64_t *dest = _dest;

	do
	{
		*dest++ = val;
	} while (--length);
}

void __memset(void *_dest, uint8_t val, size_t length)
{
	uint8_t *dest = _dest;

	do
	{
		*dest++ = val;
	} while (--length);
}
/*
 * Our memcpy can only deal with multiples of 8 bytes. This is enough for
 * simple allocator below.
 */
void __memcpy8(void *_dest, void *_src, size_t length)
{
	uint64_t *dest = _dest;
	uint64_t *src = _src;

	do
	{
		*dest++ = *src++;
	} while (--length);
}

void __memcpy(void *_dest, const void *_src, size_t length)
{
	uint8_t *dest = _dest;
	const uint8_t *src = _src;

	while (length--)
	{
		*dest++ = *src++;
	}
}

/*
 * Fast-ish clear, 8 bytes at a time.
 */
void __bzero8(void *_dest, size_t length)
{
	uint64_t *dest = _dest;

	while (length--)
	{
		*dest++ = 0;
	}
}

/*
  There are many tradeoffs in heaps. I think for Solidity, we want:
   - small code size to reduce code length
   - not many malloc objects
   - not much memory

  So I think we should avoid fragmentation by neighbour merging. The most
  costly is walking the doubly linked list looking for free space.
*/
struct chunk
{
	struct chunk *next, *prev;
	size_t length;
	size_t allocated;
};

void __init_heap()
{
	struct chunk *first = (struct chunk *)0x10000;
	first->next = first->prev = NULL;
	first->allocated = false;
	first->length = (size_t)(__builtin_wasm_memory_size(0) * 0x10000 -
							 (size_t)first - sizeof(struct chunk));
}

void __attribute__((noinline)) __free(void *m)
{
	struct chunk *cur = m;
	cur--;
	if (m)
	{
		cur->allocated = false;
		struct chunk *next = cur->next;
		if (next && !next->allocated)
		{
			// merge with next
			if ((cur->next = next->next) != NULL)
				cur->next->prev = cur;
			cur->length += next->length + sizeof(struct chunk);
			next = cur->next;
		}

		struct chunk *prev = cur->prev;
		if (prev && !prev->allocated)
		{
			// merge with previous
			prev->next = next;
			next->prev = prev;
			prev->length += cur->length + sizeof(struct chunk);
		}
	}
}

static void shrink_chunk(struct chunk *cur, size_t size)
{
	// round up to nearest 8 bytes
	size = (size + 7) & ~7;

	if (cur->length - size >= (8 + sizeof(struct chunk)))
	{
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

void *__attribute__((noinline)) __malloc(size_t size)
{
	struct chunk *cur = (struct chunk *)0x10000;

	while (cur && (cur->allocated || size > cur->length))
		cur = cur->next;

	if (cur)
	{
		shrink_chunk(cur, size);
		cur->allocated = true;
		return ++cur;
	}
	else
	{
		// go bang
		__builtin_unreachable();
	}
}

void *__realloc(void *m, size_t size)
{
	struct chunk *cur = m;

	cur--;

	struct chunk *next = cur->next;

	if (next && !next->allocated && size <= (cur->length + next->length + sizeof(struct chunk)))
	{
		// merge with next
		cur->next = next->next;
		cur->next->prev = cur;
		cur->length += next->length + sizeof(struct chunk);
		// resplit ..
		shrink_chunk(cur, size);
		return m;
	}
	else
	{
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
void __be32toleN(uint8_t *from, uint8_t *to, uint32_t length)
{
	from += 31;

	do
	{
		*to++ = *from--;
	} while (--length);
}

void __beNtoleN(uint8_t *from, uint8_t *to, uint32_t length)
{
	from += length;

	do
	{
		*to++ = *--from;
	} while (--length);
}

// This function is for used for abi encoding integers
// ABI encoding is big endian.
void __leNtobe32(uint8_t *from, uint8_t *to, uint32_t length)
{
	to += 31;

	do
	{
		*to-- = *from++;
	} while (--length);
}

void __leNtobeN(uint8_t *from, uint8_t *to, uint32_t length)
{
	to += length;

	do
	{
		*--to = *from++;
	} while (--length);
}

// sabre wants the storage keys as a hex string. Convert the uint256 pointed
// to be by v into a hex string
char *__u256ptohex(uint8_t *v, char *str)
{
	// the uint256 will be stored little endian so fill it in reverse
	str += 63;

	for (int i = 0; i < 32; i++)
	{
		uint8_t l = (v[i] & 0x0f);
		*str-- = l > 9 ? l + 'a' : '0' + l;
		uint8_t h = (v[i] >> 4);
		*str-- = h > 9 ? h + 'a' : '0' + h;
	}

	return str;
}

// Create a new vector. If initial is -1 then clear the data. This is done since a null pointer valid in wasm
struct vector *vector_new(uint32_t members, uint32_t size, uint8_t *initial)
{
	struct vector *v;
	size_t size_array = members * size;

	v = __malloc(sizeof(*v) + size_array);
	v->len = members;
	v->size = members;

	uint8_t *data = v->data;

	if ((int)initial != -1)
	{
		while (size_array--)
		{
			*data++ = *initial++;
		}
	}
	else
	{
		while (size_array--)
		{
			*data++ = 0;
		}
	}

	return v;
}

bool __memcmp(uint8_t *left, uint32_t left_len, uint8_t *right, uint32_t right_len)
{
	if (left_len != right_len)
		return false;

	while (left_len--)
	{
		if (*left++ != *right++)
			return false;
	}

	return true;
}

struct vector *concat(uint8_t *left, uint32_t left_len, uint8_t *right, uint32_t right_len)
{
	size_t size_array = left_len + right_len;
	struct vector *v = __malloc(sizeof(*v) + size_array);
	v->len = size_array;
	v->size = size_array;

	uint8_t *data = v->data;

	while (left_len--)
	{
		*data++ = *left++;
	}

	while (right_len--)
	{
		*data++ = *right++;
	}

	return v;
}
