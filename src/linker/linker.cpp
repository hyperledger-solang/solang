// SPDX-License-Identifier: Apache-2.0

// Call the LLD linker
#include "lld/Common/Driver.h"

extern "C" bool LLDWasmLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::wasm::link(args, llvm::outs(), llvm::errs(), false, false);
}

extern "C" bool LLDELFLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::elf::link(args, llvm::outs(), llvm::errs(), false, false);
}
