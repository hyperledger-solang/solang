#include "tweak.h"

/*
  *EXPERIMENTAL API*

  This module is an experimental implementation of a key tweaking protocol
  over ed25519 keys. The signature algorithm has been reimplemented from
  libsodium, but the nonce generation algorithm is *non-standard*.

  Use at your own risk
*/

static void _extension_tweak_nonce (unsigned char *nonce, const unsigned char *n,
                                 const unsigned char *m, unsigned long long mlen)
{
  // dom2(x, y) with x = 0 (not prehashed) and y = "crypto_tweak_ed25519"
  static const unsigned char TWEAK_PREFIX[32 + 2 + 20] = {
      'S', 'i', 'g', 'E', 'd', '2', '5', '5', '1', '9', ' ',
      'n', 'o', ' ', 'E', 'd', '2', '5', '5', '1', '9', ' ',
      'c', 'o', 'l', 'l', 'i', 's', 'i', 'o', 'n', 's', 0,
       20, 'c', 'r', 'y', 'p', 't', 'o', '_', 't', 'w', 'e',
      'a', 'k', '_', 'e', 'd', '2', '5', '5', '1', '9'
  };

  crypto_hash_sha512_state hs;

  crypto_hash_sha512_init(&hs);
  crypto_hash_sha512_update(&hs, TWEAK_PREFIX, sizeof TWEAK_PREFIX);
  crypto_hash_sha512_update(&hs, n, 32);
  crypto_hash_sha512_update(&hs, m, mlen);
  crypto_hash_sha512_final(&hs, nonce);
}

static inline void
_crypto_sign_ed25519_clamp(unsigned char k[32])
{
    k[0] &= 248;
    k[31] &= 127;
    k[31] |= 64;
}

static void _extension_tweak_ed25519(unsigned char *q, unsigned char *n,
                           const unsigned char *ns, unsigned long long nslen)
{
  sodium_memzero(q, sizeof q);

  crypto_hash(n, ns, nslen);
  n[31] &= 127; // clear highest bit

  crypto_scalarmult_ed25519_base_noclamp(q, n);

  // hash tweak until we get a valid tweaked q
  while (crypto_core_ed25519_is_valid_point(q) != 1) {
    crypto_hash(n, n, 32);
    n[31] &= 127; // clear highest bit

    crypto_scalarmult_ed25519_base_noclamp(q, n);
  }
}

void sn__extension_tweak_ed25519_base(unsigned char *pk, unsigned char *scalar,
                               const unsigned char *ns, unsigned long long nslen)
{
  unsigned char n64[64];

  _extension_tweak_ed25519(pk, n64, ns, nslen);

  SN_TWEAK_COPY_32(scalar, n64)
}

int sn__extension_tweak_ed25519_sign_detached(unsigned char *sig, unsigned long long *siglen_p,
                                       const unsigned char *m, unsigned long long mlen,
                                       const unsigned char *n, unsigned char *pk)
{
  crypto_hash_sha512_state hs;

  unsigned char            nonce[64];
  unsigned char            R[32];
  unsigned char            hram[64];
  unsigned char            _pk[32];

  // check if pk was passed
  if (pk == NULL) {
    pk = _pk;

    // derive pk from scalar
    if (crypto_scalarmult_ed25519_base_noclamp(pk, n) != 0) {
      return -1;
    }
  }

  _extension_tweak_nonce(nonce, n, m, mlen);
  crypto_core_ed25519_scalar_reduce(nonce, nonce);

  // R = G ^ nonce : curve point from nonce
  if (crypto_scalarmult_ed25519_base_noclamp(R, nonce) != 0) {
    return -1;
  }

  // generate challenge as h(ram) = hash(R, pk, message)
  crypto_hash_sha512_init(&hs);
  crypto_hash_sha512_update(&hs, R, 32);
  crypto_hash_sha512_update(&hs, pk, 32);
  crypto_hash_sha512_update(&hs, m, mlen);

  crypto_hash_sha512_final(&hs, hram);

  crypto_core_ed25519_scalar_reduce(hram, hram);

  // sig = nonce + n * h(ram)
  crypto_core_ed25519_scalar_mul(sig, hram, n);
  crypto_core_ed25519_scalar_add(sig + 32, nonce, sig);

  SN_TWEAK_COPY_32(sig, R)

  if (siglen_p != NULL) {
    *siglen_p = 64U;
  }

  return 0;
}

// tweak a secret key
void sn__extension_tweak_ed25519_sk_to_scalar(unsigned char *n, const unsigned char *sk)
{
  unsigned char n64[64];

  // get sk scalar from seed, cf. crypto_sign_keypair_seed
  crypto_hash(n64, sk, 32);
  _crypto_sign_ed25519_clamp(n64);

  SN_TWEAK_COPY_32(n, n64)
}

// tweak a secret key
void sn__extension_tweak_ed25519_scalar(unsigned char *scalar_out,
                                 const unsigned char *scalar,
                                 const unsigned char *ns,
                                 unsigned long long nslen)
{
  unsigned char n[64];
  unsigned char q[32];

  _extension_tweak_ed25519(q, n, ns, nslen);
  crypto_core_ed25519_scalar_add(scalar_out, scalar, n);
}

// tweak a public key
int sn__extension_tweak_ed25519_pk(unsigned char *tpk,
                                    const unsigned char *pk,
                                    const unsigned char *ns,
                                    unsigned long long nslen)
{  
  unsigned char n[64];
  unsigned char q[32];

  _extension_tweak_ed25519(q, n, ns, nslen);
  return crypto_core_ed25519_add(tpk, q, pk);
}


void sn__extension_tweak_ed25519_keypair(unsigned char *pk, unsigned char *scalar_out,
                                  unsigned char *scalar, const unsigned char *ns,
                                  unsigned long long nslen)
{
  unsigned char n64[64];

  crypto_hash(n64, ns, nslen);
  n64[31] &= 127; // clear highest bit

  sn__extension_tweak_ed25519_scalar_add(scalar_out, scalar, n64);
  crypto_scalarmult_ed25519_base_noclamp(pk, scalar_out);

  // hash tweak until we get a valid tweaked point
  while (crypto_core_ed25519_is_valid_point(pk) != 1) {
    crypto_hash(n64, n64, 32);
    n64[31] &= 127; // clear highest bit

    sn__extension_tweak_ed25519_scalar_add(scalar_out, scalar, n64);
    crypto_scalarmult_ed25519_base_noclamp(pk, scalar_out);
  }
}

// add tweak to scalar
void sn__extension_tweak_ed25519_scalar_add(unsigned char *scalar_out,
                                     const unsigned char *scalar,
                                     const unsigned char *n)
{
  crypto_core_ed25519_scalar_add(scalar_out, scalar, n);
}

// add tweak point to public key
int sn__extension_tweak_ed25519_pk_add(unsigned char *tpk,
                                const unsigned char *pk,
                                const unsigned char *q)
{
  return crypto_core_ed25519_add(tpk, pk, q);
}


int sn__extension_tweak_ed25519_keypair_add(unsigned char *pk, unsigned char *scalar_out,
                                      unsigned char *scalar, const unsigned char *tweak)
{
  sn__extension_tweak_ed25519_scalar_add(scalar_out, scalar, tweak);
  return crypto_scalarmult_ed25519_base_noclamp(pk, scalar_out);
}