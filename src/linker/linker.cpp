// SPDX-License-Identifier: Apache-2.0

// Call the LLD linker
#include "lld/Common/Driver.h"
#include "lld/Common/CommonLinkerContext.h"
#include "llvm/Support/CrashRecoveryContext.h"

// The LLD entry points are not safe to re-enter without destroying their state.
static bool DestroyCTX()
{
	llvm::CrashRecoveryContext crc;

 	return crc.RunSafely([&]() { lld::CommonLinkerContext::destroy(); });
}

extern "C" bool LLDWasmLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::wasm::link(args, llvm::outs(), llvm::errs(), false, false) && DestroyCTX();
}

extern "C" bool LLDELFLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::elf::link(args, llvm::outs(), llvm::errs(), false, false) && DestroyCTX();
}
