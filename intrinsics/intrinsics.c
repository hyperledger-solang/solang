
// clang --target=wasm32 -c -O3 -Wall intrincics.c

#include <stdint.h>

void __be32toleN(uint8_t *from, uint8_t *to, uint32_t length)
{
	from += 31;

	do {
		*to++ = *from--;
	} while (--length);
}
