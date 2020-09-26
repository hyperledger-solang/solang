// Example using incrementer.sol as generic target
//
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

// Hex printer for helping us
void dump_hex(uint8_t *data, uint32_t size)
{
	while (size--)
		printf("%02x", *data++);
}

// A solang wasm module exports these two functions
extern int solang_constructor(uint8_t *data, uint32_t size);
extern int solang_function(uint8_t *data, uint32_t size);

// These functions can be called from a solang module
void* solang_malloc(uint32_t size) {
	return malloc(size);
}

struct storage_entry {
	struct storage_entry *next;
	uint8_t key[32];
	uint32_t value_size;
	uint8_t value[0];
};

static struct storage_entry *state;

// Get the size of an element in storage. Returns 0 if the element
// does not exist. Storage elements of length 0 are not used in
// Solang.
uint32_t solang_storage_size(uint8_t key[32])
{
	struct storage_entry *entry = state;

	while (entry) {
		if (!memcmp(key, entry->key, 32))
			return entry->value_size;
		entry = entry->next;
	}

	return 0;
}

// Retrieve the storage element. The caller is assumed to know
// the size already (either fixed size or size has been retrieved
// with solang_storage_size().
void solang_storage_get(uint8_t key[32], uint8_t *data)
{
	struct storage_entry *entry = state;

	while (entry) {
		if (!memcmp(key, entry->key, 32)) {
			printf("solang_storage_get key:");
			dump_hex(entry->key, 32);
			printf(" value:");
			dump_hex(entry->value, entry->value_size);
			printf("\n");
			memcpy(data, entry->value, entry->value_size);
			return;
		}
		entry = entry->next;
	}

	printf("storage key not found\n");
}

// Delete the element from storage. This is called when the `delete`
// keyword in Solidity is used. On systems like substrate, storage
// costs rent so managing storage is important.
void solang_storage_delete(uint8_t key[32])
{
	struct storage_entry **entry = &state;

	while (*entry) {
		if (!memcmp(key, (*entry)->key, 32)) {
			struct storage_entry *sucker = *entry;
			*entry = sucker->next;
			free(sucker);
			return;
		}

		entry = &((*entry)->next);
	}
}

// Set the storage element, overwriting any previous element.
void solang_storage_set(uint8_t key[32], uint8_t *data, uint32_t size)
{
	// delete the old entry
	solang_storage_delete(key);

	printf("solang_storage_set key:");
	dump_hex(key, 32);
	printf(" value:");
	dump_hex(data, size);
	printf("\n");

	struct storage_entry *entry = malloc(sizeof(*entry) + size);

	memcpy(entry->key, key, 32);
	entry->value_size = size;
	memcpy(entry->value, data, size);

	entry->next = state;

	state = entry;
}

void solang_set_return(uint8_t *data, uint32_t size)
{
	printf("solang_return: data:");
	dump_hex(data, size);
	printf("\n");
}

// incrementer constructor expects single uint32 as argument
// ethabi encode params -l -v uint32 102 | sed 's/.\{2\}/,0x&/g'
static uint8_t constructor_arg[] = { 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x66 };
// inc expects a value to increment by
// ethabi encode function incrementer.abi inc -l -p 102 | sed 's/.\{2\}/,0x&/g'
static uint8_t inc_function_arg[] = { 0xdd,0x5d,0x52,0x11,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x66 };
// get retrieves the value
// ethabi encode function incrementer.abi get  -l  | sed 's/.\{2\}/,0x&/g'
static uint8_t get_function_arg[] = { 0x6d,0x4c,0xe6,0x3c };

int main(int argc, char *argv[])
{
	// Call the constructor. The constructor should always be called
	// on deployment of the contract, and may take arguments. Here we
	// pass in 102 as the initial value for the incrementer
	//
	printf("Calling incrementer constructor with 102 arg.\n");

	int ret = solang_constructor(constructor_arg, sizeof(constructor_arg));
	if (ret) {
		printf("error: solang_constructor returned %d\n", ret);
		exit(1);
	}

	// Call the inc() function with the argument 102
	printf("Calling incrementer function inc 102 arg.\n");
	ret = solang_function(inc_function_arg, sizeof(inc_function_arg));
	if (ret) {
		printf("error: solang_function returned %d\n", ret);
		exit(1);
	}

	// Call the get() function to retrieve the incremented value
	// Now this function returns a value. The returned data is
	// set via solang_set_return
	printf("Calling incrementer function get\n");
	ret = solang_function(get_function_arg, sizeof(get_function_arg));
	if (ret) {
		printf("error: solang_function returned %d\n", ret);
		exit(1);
	}

	return 0;
}
