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
 */

/*
 * Adapated from libsodium/crypto_pwhash/scryptsalsa208sha256/pbkdf-sha256.c
 */

#include <string.h>
#include <sodium.h>

#include "pbkdf2.h"

/**
 * pbkdf2_sha512(passwd, passwdlen, salt, saltlen, c, buf, dkLen):
 * Compute PBKDF2(passwd, salt, c, dkLen) using HMAC-SHA256 as the PRF, and
 * write the output to buf.  The value dkLen must be at most 32 * (2^32 - 1).
 */
int
sn__extension_pbkdf2_sha512(const unsigned char *passwd, size_t passwdlen,
                      const unsigned char *salt, size_t saltlen, uint64_t c,
                      unsigned char *buf, size_t dkLen)
{
    crypto_auth_hmacsha512_state PShctx, hctx;
    size_t                       i;
    unsigned char                ivec[4];
    unsigned char                U[64];
    unsigned char                T[64];
    uint64_t                     j;
    unsigned int                 k;
    size_t                       clen;

    if (dkLen > sn__extension_pbkdf2_sha512_BYTES_MAX) {
        return -1;
    }

    crypto_auth_hmacsha512_init(&PShctx, passwd, passwdlen);
    crypto_auth_hmacsha512_update(&PShctx, salt, saltlen);

    for (i = 0; i * crypto_auth_hmacsha512_BYTES < dkLen; i++) {
        SN_PBKDF2_STORE32_BE(ivec, (uint32_t)(i + 1));
        memcpy(&hctx, &PShctx, sizeof(crypto_auth_hmacsha512_state));
        crypto_auth_hmacsha512_update(&hctx, ivec, 4);
        crypto_auth_hmacsha512_final(&hctx, U);

        memcpy(T, U, crypto_auth_hmacsha512_BYTES);
        /* LCOV_EXCL_START */
        for (j = 2; j <= c; j++) {
            crypto_auth_hmacsha512_init(&hctx, passwd, passwdlen);
            crypto_auth_hmacsha512_update(&hctx, U, crypto_auth_hmacsha512_BYTES);
            crypto_auth_hmacsha512_final(&hctx, U);

            for (k = 0; k < crypto_auth_hmacsha512_BYTES; k++) {
                T[k] ^= U[k];
            }
        }
        /* LCOV_EXCL_STOP */

        clen = dkLen - i * crypto_auth_hmacsha512_BYTES;
        if (clen > crypto_auth_hmacsha512_BYTES) {
            clen = crypto_auth_hmacsha512_BYTES;
        }
        memcpy(&buf[i * crypto_auth_hmacsha512_BYTES], T, clen);
    }
    sodium_memzero((void *) &PShctx, sizeof PShctx);

    return 0;
}
