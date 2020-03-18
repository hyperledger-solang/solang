// clang --target=wasm32 -c -emit-llvm -O3 -ffreestanding -fno-builtin -Wall stdlib.c
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/*
 */
__attribute__((visibility("hidden"))) void __memset8(void *_dest, uint64_t val, size_t length)
{
	uint64_t *dest = _dest;

	do
	{
		*dest++ = val;
	} while (--length);
}

__attribute__((visibility("hidden"))) void __memset(uint8_t *dest, uint8_t val, size_t length)
{
	do
	{
		*dest++ = val;
	} while (--length);
}
/*
 * Our memcpy can only deal with multiples of 8 bytes. This is enough for
 * simple allocator below.
 */
__attribute__((visibility("hidden"))) void __memcpy8(void *_dest, void *_src, size_t length)
{
	uint64_t *dest = _dest;
	uint64_t *src = _src;

	do
	{
		*dest++ = *src++;
	} while (--length);
}

__attribute__((visibility("hidden"))) void __memcpy(uint8_t *dest, uint8_t *src, size_t length)
{
	do
	{
		*dest++ = *src++;
	} while (--length);
}

/*
 * Fast-ish clear, 8 bytes at a time.
 */
__attribute__((visibility("hidden"))) void __bzero8(void *_dest, size_t length)
{
	uint64_t *dest = _dest;

	do
		*dest++ = 0;
	while (--length);
}

/*
 * Fast-ish set, 8 bytes at a time.
 */
__attribute__((visibility("hidden"))) void __bset8(void *_dest, size_t length)
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
struct chunk
{
	struct chunk *next, *prev;
	size_t length;
	bool allocated;
};

__attribute__((visibility("hidden"))) void __init_heap()
{
	struct chunk *first = (struct chunk *)0x10000;
	first->next = first->prev = NULL;
	first->allocated = false;
	first->length = (size_t)(__builtin_wasm_memory_size(0) -
							 (size_t)first - sizeof(struct chunk));
}

__attribute__((visibility("hidden"))) void __attribute__((noinline)) __free(void *m)
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

__attribute__((visibility("hidden"))) static void shrink_chunk(struct chunk *cur, size_t size)
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

__attribute__((visibility("hidden"))) void *__attribute__((noinline)) __malloc(size_t size)
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

__attribute__((visibility("hidden"))) void *__realloc(void *m, size_t size)
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
__attribute__((visibility("hidden"))) void __be32toleN(uint8_t *from, uint8_t *to, uint32_t length)
{
	from += 31;

	do
	{
		*to++ = *from--;
	} while (--length);
}

__attribute__((visibility("hidden"))) void __beNtoleN(uint8_t *from, uint8_t *to, uint32_t length)
{
	from += length;

	do
	{
		*to++ = *--from;
	} while (--length);
}

// This function is for used for abi encoding integers
// ABI encoding is big endian.
__attribute__((visibility("hidden"))) void __leNtobe32(uint8_t *from, uint8_t *to, uint32_t length)
{
	to += 31;

	do
	{
		*to-- = *from++;
	} while (--length);
}

__attribute__((visibility("hidden"))) void __leNtobeN(uint8_t *from, uint8_t *to, uint32_t length)
{
	to += length;

	do
	{
		*--to = *from++;
	} while (--length);
}

/*
	In wasm, the instruction for multiplying two 64 bit values results in a 64 bit value. In
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
__attribute__((visibility("hidden"))) void __mul32(uint32_t left[], uint32_t right[], uint32_t out[], int len)
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

// Some compiler runtime builtins we need.

// 128 bit shift left.
typedef union {
	__uint128_t all;
	struct
	{
		uint64_t low;
		uint64_t high;
	};
} two64;

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

/*
 * Vector is used for dynamic array
 */
struct vector
{
	uint32_t len;
	uint32_t size;
	uint8_t data[];
};

__attribute__((visibility("hidden"))) struct vector *vector_new(uint32_t members, uint32_t size, uint8_t *initial)
{
	struct vector *v;
	size_t size_array = members * size;

	v = __malloc(sizeof(*v) + size_array);
	v->len = members;
	v->size = members;

	uint8_t *data = v->data;

	if (initial)
	{
		do
		{
			*data++ = *initial++;
		} while (size_array--);
	}
	else
	{
		do
		{
			*data++ = 0;
		} while (size_array--);
	}

	return v;
}

__attribute__((visibility("hidden"))) bool memcmp(uint8_t *left, uint32_t left_len, uint8_t *right, uint32_t right_len)
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

__attribute__((visibility("hidden"))) struct vector *concat(uint8_t *left, uint32_t left_len, uint8_t *right, uint32_t right_len)
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

// Encode an 32 bit integer as as scale compact integer
// https://substrate.dev/docs/en/conceptual/core/codec#vectors-lists-series-sets
uint8_t *compact_encode_u32(uint8_t *dest, uint32_t val)
{
	if (val < 64)
	{
		*dest++ = val << 2;
	}
	else if (val < 4000)
	{
		*((uint16_t *)dest) = (val << 2) | 1;
		dest += 2;
	}
	else if (val < 0x40000000)
	{
		*((uint32_t *)dest) = (val << 2) | 2;
		dest += 4;
	}
	else
	{
		*dest++ = 3;
		*((uint32_t *)dest) = val;
		dest += 4;
	}

	return dest;
}

uint8_t *scale_encode_string(uint8_t *dest, struct vector *s)
{
	uint32_t len = s->len;
	uint8_t *data_dst = compact_encode_u32(dest, len);
	uint8_t *data = s->data;

	while (len--)
	{
		*data_dst++ = *data++;
	}

	return data_dst;
}