cmake_minimum_required(VERSION 3.25)

find_package(cmake-bare REQUIRED PATHS node_modules/cmake-bare)
find_package(cmake-fetch REQUIRED PATHS node_modules/cmake-fetch)
find_package(cmake-napi REQUIRED PATHS node_modules/cmake-napi)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)

project(sodium_native C ASM)

fetch_package("github:jedisct1/libsodium#stable" SOURCE_DIR sodium)

include(TestBigEndian)

bare_target(target)

file(COPY_FILE "${sodium}/builds/msvc/version.h" "${sodium}/src/libsodium/include/sodium/version.h")

file(GLOB_RECURSE sodium_headers CONFIGURE_DEPENDS "${sodium}/src/libsodium/**/*.h")
file(GLOB_RECURSE sodium_sources CONFIGURE_DEPENDS "${sodium}/src/libsodium/**/*.c")
file(GLOB_RECURSE sodium_asm_sources CONFIGURE_DEPENDS "${sodium}/src/libsodium/**/*.S")

add_library(sodium OBJECT)

target_sources(
  sodium
  INTERFACE
    ${sodium_headers}
  PRIVATE
    ${sodium_sources}
)

target_include_directories(
  sodium
  INTERFACE
    "${sodium}/src/libsodium/include"
  PRIVATE
    "${sodium}/src/libsodium/include/sodium"
)

if(NOT target MATCHES "win32")
  target_compile_options(
    sodium
    PRIVATE
      -fvisibility=hidden
      -fno-strict-aliasing
      -fwrapv
      -flax-vector-conversions
  )
endif()

target_compile_definitions(
  sodium
  PUBLIC
    SODIUM_STATIC=1
  PRIVATE
    _GNU_SOURCE=1
    CONFIGURED=1
    DEV_MODE=1
    HAVE_ATOMIC_OPS=1
    HAVE_C11_MEMORY_FENCES=1
    HAVE_CET_H=1
    HAVE_GCC_MEMORY_FENCES=1
    HAVE_INLINE_ASM=1
    HAVE_INTTYPES_H=1
    HAVE_STDINT_H=1
    HAVE_TI_MODE=1
)

if(target MATCHES "darwin|ios")
  target_compile_definitions(
    sodium
    PRIVATE
      ASM_HIDE_SYMBOL=.private_extern
      TLS=_Thread_local
      HAVE_ARC4RANDOM=1
      HAVE_ARC4RANDOM_BUF=1
      HAVE_CATCHABLE_ABRT=1
      HAVE_CATCHABLE_SEGV=1
      HAVE_CLOCK_GETTIME=1
      HAVE_GETPID=1
      HAVE_MADVISE=1
      HAVE_MEMSET_S=1
      HAVE_MLOCK=1
      HAVE_MMAP=1
      HAVE_MPROTECT=1
      HAVE_NANOSLEEP=1
      HAVE_POSIX_MEMALIGN=1
      HAVE_PTHREAD=1
      HAVE_PTHREAD_PRIO_INHERIT=1
      HAVE_RAISE=1
      HAVE_SYSCONF=1
      HAVE_SYS_MMAN_H=1
      HAVE_SYS_PARAM_H=1
      HAVE_WEAK_SYMBOLS=1
  )

  if(NOT target MATCHES "ios")
    target_compile_definitions(
      sodium
      PRIVATE
        HAVE_GETENTROPY=1
        HAVE_SYS_RANDOM_H=1
    )
  endif()
endif()

if(target MATCHES "linux")
  target_compile_definitions(
    sodium
    PRIVATE
      ASM_HIDE_SYMBOL=.hidden
      TLS=_Thread_local
      HAVE_CATCHABLE_ABRT=1
      HAVE_CATCHABLE_SEGV=1
      HAVE_CLOCK_GETTIME=1
      HAVE_GETPID=1
      HAVE_MADVISE=1
      HAVE_MLOCK=1
      HAVE_MMAP=1
      HAVE_MPROTECT=1
      HAVE_NANOSLEEP=1
      HAVE_POSIX_MEMALIGN=1
      HAVE_PTHREAD_PRIO_INHERIT=1
      HAVE_PTHREAD=1
      HAVE_RAISE=1
      HAVE_SYSCONF=1
      HAVE_SYS_AUXV_H=1
      HAVE_SYS_MMAN_H=1
      HAVE_SYS_PARAM_H=1
      HAVE_SYS_RANDOM_H=1
      HAVE_WEAK_SYMBOLS=1
  )
endif()

if(target MATCHES "win32")
  target_compile_definitions(
    sodium
    PRIVATE
      _CRT_SECURE_NO_WARNINGS=1
      HAVE_RAISE=1
  )
endif()

if(target MATCHES "x64")
  target_compile_definitions(
    sodium
    PRIVATE
      HAVE_CPUID=1
      HAVE_RDRAND=1
      HAVE_EMMINTRIN_H=1 # SSE2
      HAVE_PMMINTRIN_H=1 # SSE3
      HAVE_TMMINTRIN_H=1 # SSSE3
      HAVE_SMMINTRIN_H=1 # SSE4.1
      HAVE_WMMINTRIN_H=1 # AES
      HAVE_AVXINTRIN_H=1 # AVX
      HAVE_AVX2INTRIN_H=1 # AVX2
      HAVE_AVX512FINTRIN_H # AVX512F
  )

  if(NOT target MATCHES "win32")
    target_compile_definitions(
      sodium
      PRIVATE
        HAVE_AMD64_ASM=1
        HAVE_AVX_ASM=1
    )

    target_sources(
      sodium
      PRIVATE
        ${sodium_asm_sources}
    )
  endif()
endif()

if(target MATCHES "arm64")
  target_compile_definitions(
    sodium
    PRIVATE
      HAVE_ARMCRYPTO=1
  )
endif()

test_big_endian(is_big_endian)

if(is_big_endian)
  target_compile_definitions(
    sodium
    PRIVATE
      NATIVE_BIG_ENDIAN=1
  )
else()
  target_compile_definitions(
    sodium
    PRIVATE
      NATIVE_LITTLE_ENDIAN=1
  )
endif()

add_bare_module(sodium_native_bare)

target_sources(
  ${sodium_native_bare}
  PRIVATE
    binding.c
    macros.h
    extensions/tweak/tweak.c
    extensions/tweak/tweak.h
    extensions/pbkdf2/pbkdf2.c
    extensions/pbkdf2/pbkdf2.h
)

target_link_libraries(
  ${sodium_native_bare}
  PRIVATE
    $<TARGET_OBJECTS:sodium>
  PUBLIC
    sodium
)

add_napi_module(sodium_native_node)

target_sources(
  ${sodium_native_node}
  PRIVATE
    binding.c
    macros.h
    extensions/tweak/tweak.c
    extensions/tweak/tweak.h
    extensions/pbkdf2/pbkdf2.c
    extensions/pbkdf2/pbkdf2.h
)

target_link_libraries(
  ${sodium_native_node}
  PRIVATE
    $<TARGET_OBJECTS:sodium>
  PUBLIC
    sodium
)
