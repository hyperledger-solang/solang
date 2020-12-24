/*
 * Vector is used for dynamic array
 */
struct vector
{
    uint32_t len;
    uint32_t size;
    uint8_t data[];
};

void *__malloc(size_t size);
void __memset(void *dest, uint8_t val, size_t length);
void __memcpy(void *dest, const void *src, size_t length);
void __memcpy8(void *_dest, void *_src, size_t length);
