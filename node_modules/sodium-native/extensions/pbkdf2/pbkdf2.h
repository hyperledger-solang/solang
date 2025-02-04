/*-
 * Copyright 2005,2007,2009 Colin Percival
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 */

/*
 * Adapated from libsodium/crypto_pwhash/scryptsalsa208sha256/pbkdf-sha256.c
 */

#include <sodium.h>

#define SN_PBKDF2_STORE32_BE(buf, n32) \
  buf[0] = n32 >> 24 & 0xff; \
  buf[1] = n32 >> 16 & 0xff; \
  buf[2] = n32 >> 8 & 0xff; \
  buf[3] = n32 >> 0 & 0xff;

#define sn__extension_pbkdf2_sha512_SALTBYTES 16U

#define sn__extension_pbkdf2_sha512_HASHBYTES crypto_hash_sha512_BYTES

#define sn__extension_pbkdf2_sha512_ITERATIONS_MIN 1U

#define sn__extension_pbkdf2_sha512_BYTES_MAX 0x3fffffffc0ULL

/**
 * extension_pbkdf2_sha512(passwd, passwdlen, salt, saltlen, c, buf, dkLen):
 * Compute PBKDF2(passwd, salt, c, dkLen) using HMAC-SHA256 as the PRF, and
 * write the output to buf.  The value dkLen must be at most 32 * (2^32 - 1).
 */
int sn__extension_pbkdf2_sha512(const unsigned char *, size_t, const unsigned char *, size_t,
                           uint64_t, unsigned char *, size_t);
