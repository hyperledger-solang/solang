#include <node_api.h>
#include <uv.h>
#include <string.h>
#include <sodium.h>
#include "macros.h"
#include "extensions/tweak/tweak.h"
#include "extensions/pbkdf2/pbkdf2.h"

static uint8_t typedarray_width (napi_typedarray_type type) {
  switch (type) {
    case napi_int8_array: return 1;
    case napi_uint8_array: return 1;
    case napi_uint8_clamped_array: return 1;
    case napi_int16_array: return 2;
    case napi_uint16_array: return 2;
    case napi_int32_array: return 4;
    case napi_uint32_array: return 4;
    case napi_float32_array: return 4;
    case napi_float64_array: return 8;
    case napi_bigint64_array: return 8;
    case napi_biguint64_array: return 8;
    default: return 0;
  }
}

static void sn_sodium_free_finalise (napi_env env, void *finalise_data, void *finalise_hint) {
  napi_adjust_external_memory(env, -4 * 4096, NULL);
  sodium_free(finalise_data);
}

napi_value sn_sodium_memzero (napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_memzero)

  SN_ARGV_TYPEDARRAY(buf, 0)

  sodium_memzero(buf_data, buf_size);

  return NULL;
}

napi_value sn_sodium_mlock (napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_mlock)

  SN_ARGV_TYPEDARRAY(buf, 0)

  SN_RETURN(sodium_mlock(buf_data, buf_size), "memory lock failed")
}

napi_value sn_sodium_munlock (napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_munlock)

  SN_ARGV_TYPEDARRAY(buf, 0)

  SN_RETURN(sodium_munlock(buf_data, buf_size), "memory unlock failed")
}

napi_value sn_sodium_free (napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_free)

  SN_ARGV_TYPEDARRAY_PTR(buf, 0)

  if (buf_data == NULL) return NULL;

  napi_value array_buf;

  SN_STATUS_THROWS(napi_get_named_property(env, argv[0], "buffer", &array_buf), "failed to get arraybuffer");
  SN_STATUS_THROWS(napi_remove_wrap(env, array_buf, NULL), "failed to remove wrap"); // remove the finalizer
  SN_STATUS_THROWS(napi_detach_arraybuffer(env, array_buf), "failed to detach array buffer");
  napi_adjust_external_memory(env, -4 * 4096, NULL);
  sodium_free(buf_data);

  return NULL;
}

napi_value sn_sodium_malloc (napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_malloc);

  SN_ARGV_UINT32(size, 0)

  void *ptr = sodium_malloc(size);

  SN_THROWS(ptr == NULL, "ENOMEM")
  napi_adjust_external_memory(env, 4 * 4096, NULL);

  SN_THROWS(ptr == NULL, "sodium_malloc failed");

  napi_value buf, value, array_buf;

  SN_STATUS_THROWS(napi_create_external_buffer(env, size, ptr, NULL, NULL, &buf), "failed to create a n-api buffer")
  SN_STATUS_THROWS(napi_get_boolean(env, true, &value), "failed to create boolean")

  SN_STATUS_THROWS(napi_get_named_property(env, buf, "buffer", &array_buf), "failed to get arraybuffer")
  SN_STATUS_THROWS(napi_set_named_property(env, buf, "secure", value), "failed to set secure property")

  SN_STATUS_THROWS(napi_wrap(env, array_buf, ptr, sn_sodium_free_finalise, NULL, NULL), "failed to wrap")

  return buf;
}

napi_value sn_sodium_mprotect_noaccess (napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_mprotect_noaccess);

  SN_ARGV_TYPEDARRAY_PTR(buf, 0)

  SN_RETURN(sodium_mprotect_noaccess(buf_data), "failed to lock buffer")
}


napi_value sn_sodium_mprotect_readonly (napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_readonly);

  SN_ARGV_TYPEDARRAY_PTR(buf, 0)

  SN_RETURN(sodium_mprotect_readonly(buf_data), "failed to unlock buffer")
}


napi_value sn_sodium_mprotect_readwrite (napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_readwrite);

  SN_ARGV_TYPEDARRAY_PTR(buf, 0)

  SN_RETURN(sodium_mprotect_readwrite(buf_data), "failed to unlock buffer")
}

napi_value sn_randombytes_random (napi_env env, napi_callback_info info) {
  napi_value result;

  napi_create_uint32(env, randombytes_random(), &result);
  return result;
}

napi_value sn_randombytes_uniform (napi_env env, napi_callback_info info) {
  SN_ARGV(1, randombytes_uniform);

  SN_ARGV_UINT32(upper_bound, 0)

  napi_value result;
  napi_create_uint32(env, randombytes_uniform(upper_bound), &result);
  return result;
}

napi_value sn_randombytes_buf (napi_env env, napi_callback_info info) {
  SN_ARGV(1, randombytes_buf)

  SN_ARGV_TYPEDARRAY(buf, 0)

  randombytes_buf(buf_data, buf_size);

  return NULL;
}

napi_value sn_randombytes_buf_deterministic (napi_env env, napi_callback_info info) {
  SN_ARGV(2, randombytes_buf)

  SN_ARGV_TYPEDARRAY(buf, 0)
  SN_ARGV_TYPEDARRAY(seed, 1)

  SN_ASSERT_LENGTH(seed_size, randombytes_SEEDBYTES, "seed")

  randombytes_buf_deterministic(buf_data, buf_size, seed_data);

  return NULL;
}

napi_value sn_sodium_memcmp(napi_env env, napi_callback_info info) {
  SN_ARGV(2, sodium_memcmp);

  SN_ARGV_TYPEDARRAY(b1, 0)
  SN_ARGV_TYPEDARRAY(b2, 1)

  SN_THROWS(b1_size != b2_size, "buffers must be of same length")

  SN_RETURN_BOOLEAN(sodium_memcmp(b1_data, b2_data, b1_size))
}

napi_value sn_sodium_increment(napi_env env, napi_callback_info info) {
  SN_ARGV(1, sodium_increment);
  SN_ARGV_TYPEDARRAY(n, 0)

  sodium_increment(n_data, n_size);

  return NULL;
}

napi_value sn_sodium_add(napi_env env, napi_callback_info info) {
  SN_ARGV(2, sodium_add);

  SN_ARGV_TYPEDARRAY(a, 0)
  SN_ARGV_TYPEDARRAY(b, 1)

  SN_THROWS(a_size != b_size, "buffers must be of same length")
  sodium_add(a_data, b_data, a_size);

  return NULL;
}

napi_value sn_sodium_sub(napi_env env, napi_callback_info info) {
  SN_ARGV(2, sodium_sub);

  SN_ARGV_TYPEDARRAY(a, 0)
  SN_ARGV_TYPEDARRAY(b, 1)

  SN_THROWS(a_size != b_size, "buffers must be of same length")
  sodium_sub(a_data, b_data, a_size);

  return NULL;
}

napi_value sn_sodium_compare(napi_env env, napi_callback_info info) {
  SN_ARGV(2, sodium_compare);

  SN_ARGV_TYPEDARRAY(a, 0)
  SN_ARGV_TYPEDARRAY(b, 1)

  SN_THROWS(a_size != b_size, "buffers must be of same length")
  int cmp = sodium_compare(a_data, b_data, a_size);

  napi_value result;
  napi_create_int32(env, cmp, &result);

  return result;
}

napi_value sn_sodium_is_zero(napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(1, 2, sodium_is_zero);

  SN_ARGV_TYPEDARRAY(a, 0)

  size_t a_full = a_size;

  if (argc == 2) {
    SN_OPT_ARGV_UINT32(a_size, 1)
    SN_THROWS(a_size > a_full, "len must be shorter than 'buf.byteLength'")
  }

  SN_RETURN_BOOLEAN_FROM_1(sodium_is_zero(a_data, a_size))
}

napi_value sn_sodium_pad(napi_env env, napi_callback_info info) {
  SN_ARGV(3, sodium_pad);

  SN_ARGV_TYPEDARRAY(buf, 0)
  SN_ARGV_UINT32(unpadded_buflen, 1)
  SN_ARGV_UINT32(blocksize, 2)

  SN_THROWS(unpadded_buflen > buf_size, "unpadded length cannot exceed buffer length")
  SN_THROWS(blocksize > buf_size, "block size cannot exceed buffer length")
  SN_THROWS(blocksize < 1, "block sizemust be at least 1 byte")
  SN_THROWS(buf_size < unpadded_buflen + (blocksize - (unpadded_buflen % blocksize)), "buf not long enough")

  napi_value result;
  size_t padded_buflen;
  sodium_pad(&padded_buflen, buf_data, unpadded_buflen, blocksize, buf_size);
  napi_create_uint32(env, padded_buflen, &result);
  return result;
}

napi_value sn_sodium_unpad(napi_env env, napi_callback_info info) {
  SN_ARGV(3, sodium_unpad);

  SN_ARGV_TYPEDARRAY(buf, 0)
  SN_ARGV_UINT32(padded_buflen, 1)
  SN_ARGV_UINT32(blocksize, 2)

  SN_THROWS(padded_buflen > buf_size, "unpadded length cannot exceed buffer length")
  SN_THROWS(blocksize > buf_size, "block size cannot exceed buffer length")
  SN_THROWS(blocksize < 1, "block size must be at least 1 byte")

  napi_value result;
  size_t unpadded_buflen;
  sodium_unpad(&unpadded_buflen, buf_data, padded_buflen, blocksize);
  napi_create_uint32(env, unpadded_buflen, &result);
  return result;
}

napi_value sn_crypto_sign_keypair(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_sign_keypair)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(sk, 1)

  SN_ASSERT_LENGTH(pk_size, crypto_sign_PUBLICKEYBYTES, "pk")
  SN_ASSERT_LENGTH(sk_size, crypto_sign_SECRETKEYBYTES, "sk")

  SN_RETURN(crypto_sign_keypair(pk_data, sk_data), "keypair generation failed")
}

napi_value sn_crypto_sign_seed_keypair(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_sign_seed_keypair)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(sk, 1)
  SN_ARGV_TYPEDARRAY(seed, 2)

  SN_ASSERT_LENGTH(pk_size, crypto_sign_PUBLICKEYBYTES, "pk")
  SN_ASSERT_LENGTH(sk_size, crypto_sign_SECRETKEYBYTES, "sk")
  SN_ASSERT_LENGTH(seed_size, crypto_sign_SEEDBYTES, "seed")

  SN_RETURN(crypto_sign_seed_keypair(pk_data, sk_data, seed_data), "keypair generation failed")
}

napi_value sn_crypto_sign(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_sign)

  SN_ARGV_TYPEDARRAY(sm, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(sk, 2)

  SN_THROWS(sm_size != crypto_sign_BYTES + m_size, "sm must be 'm.byteLength + crypto_sign_BYTES' bytes")
  SN_ASSERT_LENGTH(sk_size, crypto_sign_SECRETKEYBYTES, "sk")

  SN_RETURN(crypto_sign(sm_data, NULL, m_data, m_size, sk_data), "signature failed")
}

napi_value sn_crypto_sign_open(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_sign_open)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_TYPEDARRAY(sm, 1)
  SN_ARGV_TYPEDARRAY(pk, 2)

  SN_THROWS(m_size != sm_size - crypto_sign_BYTES, "m must be 'sm.byteLength - crypto_sign_BYTES' bytes")
  SN_ASSERT_MIN_LENGTH(sm_size, crypto_sign_BYTES, "sm")
  SN_ASSERT_LENGTH(pk_size, crypto_sign_PUBLICKEYBYTES, "pk")

  SN_RETURN_BOOLEAN(crypto_sign_open(m_data, NULL, sm_data, sm_size, pk_data))
}

napi_value sn_crypto_sign_detached(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_sign_detached)

  SN_ARGV_TYPEDARRAY(sig, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(sk, 2)

  SN_ASSERT_LENGTH(sig_size, crypto_sign_BYTES, "sm")
  SN_ASSERT_LENGTH(sk_size, crypto_sign_SECRETKEYBYTES, "sk")

  SN_RETURN(crypto_sign_detached(sig_data, NULL, m_data, m_size, sk_data), "signature failed")
}

napi_value sn_crypto_sign_verify_detached(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_sign_verify_detached)

  SN_ARGV_TYPEDARRAY(sig, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(pk, 2)

  SN_ASSERT_MIN_LENGTH(sig_size, crypto_sign_BYTES, "sig")
  SN_ASSERT_LENGTH(pk_size, crypto_sign_PUBLICKEYBYTES, "pk")

  SN_RETURN_BOOLEAN(crypto_sign_verify_detached(sig_data, m_data, m_size, pk_data))
}

napi_value sn_crypto_sign_ed25519_sk_to_pk(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_sign_ed25519_sk_to_pk)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(sk, 1)

  SN_ASSERT_LENGTH(pk_size, crypto_sign_PUBLICKEYBYTES, "pk")
  SN_ASSERT_LENGTH(sk_size, crypto_sign_SECRETKEYBYTES, "sk")

  SN_RETURN(crypto_sign_ed25519_sk_to_pk(pk_data, sk_data), "public key generation failed")
}

napi_value sn_crypto_sign_ed25519_pk_to_curve25519(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_sign_ed25519_sk_to_pk)

  SN_ARGV_TYPEDARRAY(x25519_pk, 0)
  SN_ARGV_TYPEDARRAY(ed25519_pk, 1)

  SN_ASSERT_LENGTH(x25519_pk_size, crypto_box_PUBLICKEYBYTES, "x25519_pk")
  SN_ASSERT_LENGTH(ed25519_pk_size, crypto_sign_PUBLICKEYBYTES, "ed25519_pk")

  SN_RETURN(crypto_sign_ed25519_pk_to_curve25519(x25519_pk_data, ed25519_pk_data), "public key conversion failed")
}

napi_value sn_crypto_sign_ed25519_sk_to_curve25519(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_sign_ed25519_sk_to_pk)

  SN_ARGV_TYPEDARRAY(x25519_sk, 0)
  SN_ARGV_TYPEDARRAY(ed25519_sk, 1)

  SN_ASSERT_LENGTH(x25519_sk_size, crypto_box_SECRETKEYBYTES, "x25519_sk")
  SN_THROWS(ed25519_sk_size != crypto_sign_SECRETKEYBYTES && ed25519_sk_size != crypto_box_SECRETKEYBYTES, "ed25519_sk should either be 'crypto_sign_SECRETKEYBYTES' bytes or 'crypto_sign_SECRETKEYBYTES - crypto_sign_PUBLICKEYBYTES' bytes")

  SN_RETURN(crypto_sign_ed25519_sk_to_curve25519(x25519_sk_data, ed25519_sk_data), "secret key conversion failed")
}

napi_value sn_crypto_generichash(napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(2, 3, crypto_generichash)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(in, 1)

  SN_ASSERT_MIN_LENGTH(out_size, crypto_generichash_BYTES_MIN, "out")
  SN_ASSERT_MAX_LENGTH(out_size, crypto_generichash_BYTES_MAX, "out")

  void *key_data = NULL;
  size_t key_size = 0;

  if (argc == 3) {
    SN_OPT_ARGV_TYPEDARRAY(key, 2)
    SN_THROWS(key_size < crypto_generichash_KEYBYTES_MIN, "key")
    SN_ASSERT_MIN_LENGTH(key_size, crypto_generichash_KEYBYTES_MIN, "key")
    SN_ASSERT_MAX_LENGTH(key_size, crypto_generichash_KEYBYTES_MAX, "key")
  }

  SN_RETURN(crypto_generichash(out_data, out_size, in_data, in_size, key_data, key_size), "hash failed")
}

napi_value sn_crypto_generichash_batch(napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(2, 3, crypto_generichash)

  SN_ARGV_TYPEDARRAY(out, 0)

  uint32_t batch_length;
  napi_get_array_length(env, argv[1], &batch_length);

  SN_ASSERT_MIN_LENGTH(out_size, crypto_generichash_BYTES_MIN, "out")
  SN_ASSERT_MAX_LENGTH(out_size, crypto_generichash_BYTES_MAX, "out")

  void *key_data = NULL;
  size_t key_size = 0;

  if (argc == 3) {
    SN_OPT_ARGV_TYPEDARRAY(key, 2)
    SN_ASSERT_MIN_LENGTH(key_size, crypto_generichash_KEYBYTES_MIN, "key")
    SN_ASSERT_MAX_LENGTH(key_size, crypto_generichash_KEYBYTES_MAX, "key")
  }

  crypto_generichash_state state;
  crypto_generichash_init(&state, key_data, key_size, out_size);
  for (uint32_t i = 0; i < batch_length; i++) {
    napi_value element;
    napi_get_element(env, argv[1], i, &element);
    SN_TYPEDARRAY_ASSERT(buf, element, "batch element should be passed as a TypedArray")
    SN_TYPEDARRAY(buf, element)
    crypto_generichash_update(&state, buf_data, buf_size);
  }

  SN_RETURN(crypto_generichash_final(&state, out_data, out_size), "batch failed")
}

napi_value sn_crypto_generichash_keygen(napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_generichash_keygen)

  SN_ARGV_TYPEDARRAY(key, 0)

  SN_ASSERT_LENGTH(key_size, crypto_generichash_KEYBYTES, "key")

  crypto_generichash_keygen(key_data);

  return NULL;
}

napi_value sn_crypto_generichash_init(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_generichash_init)

  SN_ARGV_BUFFER_CAST(crypto_generichash_state *, state, 0)
  SN_ARGV_OPTS_TYPEDARRAY(key, 1)
  SN_ARGV_UINT32(outlen, 2)

  SN_THROWS(state_size != sizeof(crypto_generichash_state), "state must be 'crypto_generichash_STATEBYTES' bytes")

  if (key_data != NULL) {
    SN_ASSERT_MIN_LENGTH(key_size, crypto_generichash_KEYBYTES_MIN, "key")
    SN_ASSERT_MAX_LENGTH(key_size, crypto_generichash_KEYBYTES_MAX, "key")
  }

  SN_RETURN(crypto_generichash_init(state, key_data, key_size, outlen), "hash failed to initialise")
}

napi_value sn_crypto_generichash_update(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_generichash_update)

  SN_ARGV_BUFFER_CAST(crypto_generichash_state *, state, 0)
  SN_ARGV_TYPEDARRAY(in, 1)

  SN_THROWS(state_size != sizeof(crypto_generichash_state), "state must be 'crypto_generichash_STATEBYTES' bytes")

  SN_RETURN(crypto_generichash_update(state, in_data, in_size), "update failed")
}

napi_value sn_crypto_generichash_final(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_generichash_final)

  SN_ARGV_BUFFER_CAST(crypto_generichash_state *, state, 0)
  SN_ARGV_TYPEDARRAY(out, 1)

  SN_THROWS(state_size != sizeof(crypto_generichash_state), "state must be 'crypto_generichash_STATEBYTES' bytes")

  SN_RETURN(crypto_generichash_final(state, out_data, out_size), "digest failed")
}

napi_value sn_crypto_box_keypair(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_box_keypair)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(sk, 1)

  SN_ASSERT_LENGTH(pk_size, crypto_box_PUBLICKEYBYTES, "pk")
  SN_ASSERT_LENGTH(sk_size, crypto_box_SECRETKEYBYTES, "sk")

  SN_RETURN(crypto_box_keypair(pk_data, sk_data), "keypair generation failed")
}

napi_value sn_crypto_box_seed_keypair(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_box_seed_keypair)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(sk, 1)
  SN_ARGV_TYPEDARRAY(seed, 2)

  SN_ASSERT_LENGTH(pk_size, crypto_box_PUBLICKEYBYTES, "pk")
  SN_ASSERT_LENGTH(sk_size, crypto_box_SECRETKEYBYTES, "sk")
  SN_ASSERT_LENGTH(seed_size, crypto_box_SEEDBYTES, "seed")

  SN_RETURN(crypto_box_seed_keypair(pk_data, sk_data, seed_data), "keypair generation failed")
}

napi_value sn_crypto_box_easy(napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_box_easy)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(pk, 3)
  SN_ARGV_TYPEDARRAY(sk, 4)

  SN_THROWS(c_size != m_size + crypto_box_MACBYTES, "c must be 'm.byteLength + crypto_box_MACBYTES' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_box_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(sk_size, crypto_box_SECRETKEYBYTES, "sk")
  SN_ASSERT_LENGTH(pk_size, crypto_box_PUBLICKEYBYTES, "pk")

  SN_RETURN(crypto_box_easy(c_data, m_data, m_size, n_data, pk_data, sk_data), "crypto box failed")
}

napi_value sn_crypto_box_open_easy(napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_box_open_easy)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_TYPEDARRAY(c, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(pk, 3)
  SN_ARGV_TYPEDARRAY(sk, 4)

  SN_THROWS(m_size != c_size - crypto_box_MACBYTES, "m must be 'c.byteLength - crypto_box_MACBYTES' bytes")
  SN_ASSERT_MIN_LENGTH(c_size, crypto_box_MACBYTES, "c")
  SN_ASSERT_LENGTH(n_size, crypto_box_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(sk_size, crypto_box_SECRETKEYBYTES, "sk")
  SN_ASSERT_LENGTH(pk_size, crypto_box_PUBLICKEYBYTES, "pk")

  SN_RETURN_BOOLEAN(crypto_box_open_easy(m_data, c_data, c_size, n_data, pk_data, sk_data))
}

napi_value sn_crypto_box_detached(napi_env env, napi_callback_info info) {
  SN_ARGV(6, crypto_box_detached)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(mac, 1)
  SN_ARGV_TYPEDARRAY(m, 2)
  SN_ARGV_TYPEDARRAY(n, 3)
  SN_ARGV_TYPEDARRAY(pk, 4)
  SN_ARGV_TYPEDARRAY(sk, 5)

  SN_THROWS(c_size != m_size, "c must be 'm.byteLength' bytes")
  SN_ASSERT_LENGTH(mac_size, crypto_box_MACBYTES, "mac")
  SN_ASSERT_LENGTH(n_size, crypto_box_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(sk_size, crypto_box_SECRETKEYBYTES, "sk")
  SN_ASSERT_LENGTH(pk_size, crypto_box_PUBLICKEYBYTES, "pk")

  SN_RETURN(crypto_box_detached(c_data, mac_data, m_data, m_size, n_data, pk_data, sk_data), "signature failed")
}

napi_value sn_crypto_box_open_detached(napi_env env, napi_callback_info info) {
  SN_ARGV(6, crypto_box_open_detached)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_TYPEDARRAY(c, 1)
  SN_ARGV_TYPEDARRAY(mac, 2)
  SN_ARGV_TYPEDARRAY(n, 3)
  SN_ARGV_TYPEDARRAY(pk, 4)
  SN_ARGV_TYPEDARRAY(sk, 5)

  SN_THROWS(m_size != c_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(mac_size, crypto_box_MACBYTES, "mac")
  SN_ASSERT_LENGTH(n_size, crypto_box_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(sk_size, crypto_box_SECRETKEYBYTES, "sk")
  SN_ASSERT_LENGTH(pk_size, crypto_box_PUBLICKEYBYTES, "pk")

  SN_RETURN_BOOLEAN(crypto_box_open_detached(m_data, c_data, mac_data, c_size, n_data, pk_data, sk_data))
}

napi_value sn_crypto_box_seal(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_box_seal)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(pk, 2)

  SN_THROWS(c_size != m_size + crypto_box_SEALBYTES, "c must be 'm.byteLength + crypto_box_SEALBYTES' bytes")
  SN_ASSERT_LENGTH(pk_size, crypto_box_PUBLICKEYBYTES, "pk")

  SN_RETURN(crypto_box_seal(c_data, m_data, m_size, pk_data), "failed to create seal")
}

napi_value sn_crypto_box_seal_open(napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_box_seal_open)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_TYPEDARRAY(c, 1)
  SN_ARGV_TYPEDARRAY(pk, 2)
  SN_ARGV_TYPEDARRAY(sk, 3)

  SN_THROWS(m_size != c_size - crypto_box_SEALBYTES, "m must be 'c.byteLength - crypto_box_SEALBYTES' bytes")
  SN_ASSERT_MIN_LENGTH(c_size, crypto_box_SEALBYTES, "c")
  SN_ASSERT_LENGTH(sk_size, crypto_box_SECRETKEYBYTES, "sk")
  SN_ASSERT_LENGTH(pk_size, crypto_box_PUBLICKEYBYTES, "pk")

  SN_RETURN_BOOLEAN(crypto_box_seal_open(m_data, c_data, c_size, pk_data, sk_data))
}

napi_value sn_crypto_secretbox_easy(napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_secretbox_easy)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(k, 3)

  SN_THROWS(c_size != m_size + crypto_secretbox_MACBYTES, "c must be 'm.byteLength + crypto_secretbox_MACBYTES' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_secretbox_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_secretbox_KEYBYTES, "k")

  SN_RETURN(crypto_secretbox_easy(c_data, m_data, m_size, n_data, k_data), "crypto secretbox failed")
}

napi_value sn_crypto_secretbox_open_easy(napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_secretbox_open_easy)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_TYPEDARRAY(c, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(k, 3)

  SN_THROWS(m_size != c_size - crypto_secretbox_MACBYTES, "m must be 'c - crypto_secretbox_MACBYTES' bytes")
  SN_ASSERT_MIN_LENGTH(c_size, crypto_secretbox_MACBYTES, "c")
  SN_ASSERT_LENGTH(n_size, crypto_secretbox_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_secretbox_KEYBYTES, "k")

  SN_RETURN_BOOLEAN(crypto_secretbox_open_easy(m_data, c_data, c_size, n_data, k_data))
}

napi_value sn_crypto_secretbox_detached(napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_secretbox_detached)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(mac, 1)
  SN_ARGV_TYPEDARRAY(m, 2)
  SN_ARGV_TYPEDARRAY(n, 3)
  SN_ARGV_TYPEDARRAY(k, 4)

  SN_THROWS(c_size != m_size, "c must 'm.byteLength' bytes")
  SN_ASSERT_LENGTH(mac_size, crypto_secretbox_MACBYTES, "mac")
  SN_ASSERT_LENGTH(n_size, crypto_secretbox_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_secretbox_KEYBYTES, "k")

  SN_RETURN(crypto_secretbox_detached(c_data, mac_data, m_data, m_size, n_data, k_data), "failed to open box")
}

napi_value sn_crypto_secretbox_open_detached(napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_secretbox_open_detached)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_TYPEDARRAY(c, 1)
  SN_ARGV_TYPEDARRAY(mac, 2)
  SN_ARGV_TYPEDARRAY(n, 3)
  SN_ARGV_TYPEDARRAY(k, 4)

  SN_THROWS(m_size != c_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(mac_size, crypto_secretbox_MACBYTES, "mac")
  SN_ASSERT_LENGTH(n_size, crypto_secretbox_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_secretbox_KEYBYTES, "k")

  SN_RETURN_BOOLEAN(crypto_secretbox_open_detached(m_data, c_data, mac_data, c_size, n_data, k_data))
}

napi_value sn_crypto_stream(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(n_size, crypto_stream_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_KEYBYTES, "k")

  SN_RETURN(crypto_stream(c_data, c_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_xor(napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_stream_xor)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(k, 3)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_KEYBYTES, "k")

  SN_RETURN(crypto_stream_xor(c_data, m_data, m_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_chacha20(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_chacha20)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(n_size, crypto_stream_chacha20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_chacha20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_chacha20(c_data, c_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_chacha20_xor (napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_stream_chacha20_xor)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(k, 3)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_chacha20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_chacha20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_chacha20_xor(c_data, m_data, m_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_chacha20_xor_ic(napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_stream_chacha20_xor_ic)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_UINT32(ic, 3)
  SN_ARGV_TYPEDARRAY(k, 4)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_chacha20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_chacha20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_chacha20_xor_ic(c_data, m_data, m_size, n_data, ic, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_chacha20_ietf(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_chacha20_ietf)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(n_size, crypto_stream_chacha20_ietf_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_chacha20_ietf_KEYBYTES, "k")

  SN_RETURN(crypto_stream_chacha20_ietf(c_data, c_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_chacha20_ietf_xor(napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_stream_chacha20_ietf_xor)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(k, 3)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_chacha20_ietf_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_chacha20_ietf_KEYBYTES, "k")

  SN_RETURN(crypto_stream_chacha20_ietf_xor(c_data, m_data, m_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_chacha20_ietf_xor_ic(napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_stream_chacha20_ietf_xor_ic)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_UINT32(ic, 3)
  SN_ARGV_TYPEDARRAY(k, 4)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_chacha20_ietf_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_chacha20_ietf_KEYBYTES, "k")

  SN_RETURN(crypto_stream_chacha20_ietf_xor_ic(c_data, m_data, m_size, n_data, ic, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_xchacha20(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_xchacha20)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(n_size, crypto_stream_xchacha20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_xchacha20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_xchacha20(c_data, c_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_xchacha20_xor (napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_stream_xchacha20_xor)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(k, 3)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_xchacha20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_xchacha20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_xchacha20_xor(c_data, m_data, m_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_xchacha20_xor_ic(napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_stream_xchacha20_xor_ic)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_UINT32(ic, 3)
  SN_ARGV_TYPEDARRAY(k, 4)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_xchacha20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_xchacha20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_xchacha20_xor_ic(c_data, m_data, m_size, n_data, ic, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_salsa20(napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_salsa20)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(n_size, crypto_stream_salsa20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_salsa20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_salsa20(c_data, c_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_salsa20_xor (napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_stream_salsa20_xor)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_TYPEDARRAY(k, 3)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_salsa20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_salsa20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_salsa20_xor(c_data, m_data, m_size, n_data, k_data), "stream encryption failed")
}

napi_value sn_crypto_stream_salsa20_xor_ic(napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_stream_salsa20_xor_ic)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(n, 2)
  SN_ARGV_UINT32(ic, 3)
  SN_ARGV_TYPEDARRAY(k, 4)

  SN_THROWS(c_size != m_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_salsa20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_salsa20_KEYBYTES, "k")

  SN_RETURN(crypto_stream_salsa20_xor_ic(c_data, m_data, m_size, n_data, ic, k_data), "stream encryption failed")
}

napi_value sn_crypto_auth (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_auth)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(in, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(out_size, crypto_auth_BYTES, "out")
  SN_ASSERT_LENGTH(k_size, crypto_auth_KEYBYTES, "k")

  SN_RETURN(crypto_auth(out_data, in_data, in_size, k_data), "failed to generate authentication tag")
}

napi_value sn_crypto_auth_verify (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_auth_verify)

  SN_ARGV_TYPEDARRAY(h, 0)
  SN_ARGV_TYPEDARRAY(in, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(h_size, crypto_auth_BYTES, "h")
  SN_ASSERT_LENGTH(k_size, crypto_auth_KEYBYTES, "k")

  SN_RETURN_BOOLEAN(crypto_auth_verify(h_data, in_data, in_size, k_data))
}

napi_value sn_crypto_onetimeauth (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_onetimeauth)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(in, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(out_size, crypto_onetimeauth_BYTES, "out")
  SN_ASSERT_LENGTH(k_size, crypto_onetimeauth_KEYBYTES, "k")

  SN_RETURN(crypto_onetimeauth(out_data, in_data, in_size, k_data), "failed to generate onetime authentication tag")
}

napi_value sn_crypto_onetimeauth_init (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_onetimeauth_init)

  SN_ARGV_BUFFER_CAST(crypto_onetimeauth_state *, state, 0)
  SN_ARGV_TYPEDARRAY(k, 1)

  SN_THROWS(state_size != sizeof(crypto_onetimeauth_state), "state must be 'crypto_onetimeauth_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(k_size, crypto_onetimeauth_KEYBYTES, "k")

  SN_RETURN(crypto_onetimeauth_init(state, k_data), "failed to initialise onetime authentication")
}

napi_value sn_crypto_onetimeauth_update(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_onetimeauth_update)

  SN_ARGV_BUFFER_CAST(crypto_onetimeauth_state *, state, 0)
  SN_ARGV_TYPEDARRAY(in, 1)

  SN_THROWS(state_size != sizeof(crypto_onetimeauth_state), "state must be 'crypto_onetimeauth_STATEBYTES' bytes")

  SN_RETURN(crypto_onetimeauth_update(state, in_data, in_size), "update failed")
}

napi_value sn_crypto_onetimeauth_final(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_onetimeauth_final)

  SN_ARGV_BUFFER_CAST(crypto_onetimeauth_state *, state, 0)
  SN_ARGV_TYPEDARRAY(out, 1)

  SN_THROWS(state_size != sizeof(crypto_onetimeauth_state), "state must be 'crypto_onetimeauth_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(out_size, crypto_onetimeauth_BYTES, "out")

  SN_RETURN(crypto_onetimeauth_final(state, out_data), "failed to generate authentication tag")
}

napi_value sn_crypto_onetimeauth_verify (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_onetimeauth_verify)

  SN_ARGV_TYPEDARRAY(h, 0)
  SN_ARGV_TYPEDARRAY(in, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(h_size, crypto_onetimeauth_BYTES, "h")
  SN_ASSERT_LENGTH(k_size, crypto_onetimeauth_KEYBYTES, "k")

  SN_RETURN_BOOLEAN(crypto_onetimeauth_verify(h_data, in_data, in_size, k_data))
}

// CHECK: memlimit can be >32bit
napi_value sn_crypto_pwhash (napi_env env, napi_callback_info info) {
  SN_ARGV(6, crypto_pwhash)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(passwd, 1)
  SN_ARGV_TYPEDARRAY(salt, 2)
  SN_ARGV_UINT64(opslimit, 3)
  SN_ARGV_UINT64(memlimit, 4)
  SN_ARGV_UINT8(alg, 5)

  SN_ASSERT_MIN_LENGTH(out_size, crypto_pwhash_BYTES_MIN, "out")
  SN_ASSERT_MAX_LENGTH(out_size, crypto_pwhash_BYTES_MAX, "out")
  SN_ASSERT_LENGTH(salt_size, crypto_pwhash_SALTBYTES, "salt")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_MEMLIMIT_MAX, "memlimit")
  SN_THROWS(alg < 1 || alg > 2, "alg must be either Argon2i 1.3 or Argon2id 1.3")

  SN_RETURN(crypto_pwhash(out_data, out_size, passwd_data, passwd_size, salt_data, opslimit, memlimit, alg), "password hashing failed, check memory requirements.")
}

napi_value sn_crypto_pwhash_str (napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_pwhash_str)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(passwd, 1)
  SN_ARGV_UINT64(opslimit, 2)
  SN_ARGV_UINT64(memlimit, 3)

  SN_ASSERT_LENGTH(out_size, crypto_pwhash_STRBYTES, "out")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_MEMLIMIT_MAX, "memlimit")

  SN_RETURN(crypto_pwhash_str(out_data, passwd_data, passwd_size, opslimit, memlimit), "password hashing failed, check memory requirements.")
}

napi_value sn_crypto_pwhash_str_verify (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_pwhash_str_verify)

  SN_ARGV_TYPEDARRAY(str, 0)
  SN_ARGV_TYPEDARRAY(passwd, 1)

  SN_ASSERT_LENGTH(str_size, crypto_pwhash_STRBYTES, "str")

  SN_RETURN_BOOLEAN(crypto_pwhash_str_verify(str_data, passwd_data, passwd_size))
}

// CHECK: returns 1, 0, -1
napi_value sn_crypto_pwhash_str_needs_rehash (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_pwhash_str_needs_rehash)

  SN_ARGV_TYPEDARRAY(str, 0)
  SN_ARGV_UINT64(opslimit, 1)
  SN_ARGV_UINT64(memlimit, 2)

  SN_ASSERT_LENGTH(str_size, crypto_pwhash_STRBYTES, "str")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_MEMLIMIT_MAX, "memlimit")

  SN_RETURN_BOOLEAN_FROM_1(crypto_pwhash_str_needs_rehash(str_data, opslimit, memlimit))
}

// CHECK: memlimit can be >32bit
napi_value sn_crypto_pwhash_scryptsalsa208sha256 (napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_pwhash_scryptsalsa208sha256)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(passwd, 1)
  SN_ARGV_TYPEDARRAY(salt, 2)
  SN_ARGV_UINT64(opslimit, 3)
  SN_ARGV_UINT64(memlimit, 4)

  SN_ASSERT_MIN_LENGTH(out_size, crypto_pwhash_scryptsalsa208sha256_BYTES_MIN, "out")
  SN_ASSERT_MAX_LENGTH(out_size, crypto_pwhash_scryptsalsa208sha256_BYTES_MAX, "out")
  SN_ASSERT_LENGTH(salt_size, crypto_pwhash_scryptsalsa208sha256_SALTBYTES, "salt")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX, "memlimit")

  SN_RETURN(crypto_pwhash_scryptsalsa208sha256(out_data, out_size, passwd_data, passwd_size, salt_data, opslimit, memlimit), "password hashing failed, check memory requirements.")
}

napi_value sn_crypto_pwhash_scryptsalsa208sha256_str (napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_pwhash_scryptsalsa208sha256_str)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(passwd, 1)
  SN_ARGV_UINT64(opslimit, 2)
  SN_ARGV_UINT64(memlimit, 3)

  SN_ASSERT_LENGTH(out_size, crypto_pwhash_scryptsalsa208sha256_STRBYTES, "out")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX, "memlimit")

  SN_RETURN(crypto_pwhash_scryptsalsa208sha256_str(out_data, passwd_data, passwd_size, opslimit, memlimit), "password hashing failed, check memory requirements.")
}

napi_value sn_crypto_pwhash_scryptsalsa208sha256_str_verify (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_pwhash_scryptsalsa208sha256_str_verify)

  SN_ARGV_TYPEDARRAY(str, 0)
  SN_ARGV_TYPEDARRAY(passwd, 1)

  SN_ASSERT_LENGTH(str_size, crypto_pwhash_scryptsalsa208sha256_STRBYTES, "str")

  SN_RETURN_BOOLEAN(crypto_pwhash_scryptsalsa208sha256_str_verify(str_data, passwd_data, passwd_size))
}

napi_value sn_crypto_pwhash_scryptsalsa208sha256_str_needs_rehash (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_pwhash_scryptsalsa208sha256_str_needs_rehash)

  SN_ARGV_TYPEDARRAY(str, 0)
  SN_ARGV_UINT64(opslimit, 1)
  SN_ARGV_UINT64(memlimit, 2)

  SN_ASSERT_LENGTH(str_size, crypto_pwhash_scryptsalsa208sha256_STRBYTES, "str")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_MEMLIMIT_MAX, "memlimit")

  SN_RETURN_BOOLEAN_FROM_1(crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(str_data, opslimit, memlimit))
}

napi_value sn_crypto_kx_keypair (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_kx_keypair)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(sk, 1)

  SN_ASSERT_LENGTH(pk_size, crypto_kx_PUBLICKEYBYTES, "pk")
  SN_ASSERT_LENGTH(sk_size, crypto_kx_SECRETKEYBYTES, "sk")

  SN_RETURN(crypto_kx_keypair(pk_data, sk_data), "failed to generate keypair")
}

napi_value sn_crypto_kx_seed_keypair (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_kx_seed_keypair)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(sk, 1)
  SN_ARGV_TYPEDARRAY(seed, 2)

  SN_ASSERT_LENGTH(pk_size, crypto_kx_PUBLICKEYBYTES, "pk")
  SN_ASSERT_LENGTH(sk_size, crypto_kx_SECRETKEYBYTES, "sk")
  SN_ASSERT_LENGTH(seed_size, crypto_kx_SEEDBYTES, "seed")

  SN_RETURN(crypto_kx_seed_keypair(pk_data, sk_data, seed_data), "failed to derive keypair from seed")
}

napi_value sn_crypto_kx_client_session_keys (napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_kx_client_session_keys)

  SN_ARGV_OPTS_TYPEDARRAY(rx, 0)
  SN_ARGV_OPTS_TYPEDARRAY(tx, 1)

  SN_THROWS(rx_data == NULL && tx_data == NULL, "at least one session key must be specified")

  SN_ARGV_TYPEDARRAY(client_pk, 2)
  SN_ARGV_TYPEDARRAY(client_sk, 3)
  SN_ARGV_TYPEDARRAY(server_pk, 4)

  SN_ASSERT_LENGTH(client_pk_size, crypto_kx_PUBLICKEYBYTES, "client_pk")
  SN_ASSERT_LENGTH(client_sk_size, crypto_kx_SECRETKEYBYTES, "client_sk")
  SN_ASSERT_LENGTH(server_pk_size, crypto_kx_PUBLICKEYBYTES, "server_pk")

  SN_THROWS(tx_size != crypto_kx_SESSIONKEYBYTES && tx_data != NULL, "transmitting key buffer must be 'crypto_kx_SESSIONKEYBYTES' bytes or null")
  SN_THROWS(rx_size != crypto_kx_SESSIONKEYBYTES && rx_data != NULL, "receiving key buffer must be 'crypto_kx_SESSIONKEYBYTES' bytes or null")

  SN_RETURN(crypto_kx_client_session_keys(rx_data, tx_data, client_pk_data, client_sk_data, server_pk_data), "failed to derive session keys")
}

napi_value sn_crypto_kx_server_session_keys (napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_kx_server_session_keys)

  SN_ARGV_OPTS_TYPEDARRAY(rx, 0)
  SN_ARGV_OPTS_TYPEDARRAY(tx, 1)

  SN_THROWS(rx_data == NULL && tx_data == NULL, "at least one session key must be specified")

  SN_ARGV_TYPEDARRAY(server_pk, 2)
  SN_ARGV_TYPEDARRAY(server_sk, 3)
  SN_ARGV_TYPEDARRAY(client_pk, 4)

  SN_ASSERT_LENGTH(server_pk_size, crypto_kx_PUBLICKEYBYTES, "server_pk")
  SN_ASSERT_LENGTH(server_sk_size, crypto_kx_SECRETKEYBYTES, "server_sk")
  SN_ASSERT_LENGTH(client_pk_size, crypto_kx_PUBLICKEYBYTES, "client_pk")

  SN_THROWS(tx_size != crypto_kx_SESSIONKEYBYTES && tx_data != NULL, "transmitting key buffer must be 'crypto_kx_SESSIONKEYBYTES' bytes or null")
  SN_THROWS(rx_size != crypto_kx_SESSIONKEYBYTES && rx_data != NULL, "receiving key buffer must be 'crypto_kx_SESSIONKEYBYTES' bytes or null")

  SN_RETURN(crypto_kx_server_session_keys(rx_data, tx_data, server_pk_data, server_sk_data, client_pk_data), "failed to derive session keys")
}

napi_value sn_crypto_scalarmult_base (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_scalarmult_base)

  SN_ARGV_TYPEDARRAY(q, 0)
  SN_ARGV_TYPEDARRAY(n, 1)

  SN_ASSERT_LENGTH(q_size, crypto_scalarmult_BYTES, "q")
  SN_ASSERT_LENGTH(n_size, crypto_scalarmult_SCALARBYTES, "n")

  SN_RETURN(crypto_scalarmult_base(q_data, n_data), "failed to derive public key")
}

napi_value sn_crypto_scalarmult (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_scalarmult)

  SN_ARGV_TYPEDARRAY(q, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(p, 2)

  SN_ASSERT_LENGTH(q_size, crypto_scalarmult_BYTES, "q")
  SN_ASSERT_LENGTH(n_size, crypto_scalarmult_SCALARBYTES, "n")
  SN_ASSERT_LENGTH(p_size, crypto_scalarmult_BYTES, "p")

  SN_RETURN(crypto_scalarmult(q_data, n_data, p_data), "failed to derive shared secret")
}

napi_value sn_crypto_scalarmult_ed25519_base (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_scalarmult_ed25519_base)

  SN_ARGV_TYPEDARRAY(q, 0)
  SN_ARGV_TYPEDARRAY(n, 1)

  SN_ASSERT_LENGTH(q_size, crypto_scalarmult_ed25519_BYTES, "q")
  SN_ASSERT_LENGTH(n_size, crypto_scalarmult_ed25519_SCALARBYTES, "n")

  SN_RETURN(crypto_scalarmult_ed25519_base(q_data, n_data), "failed to derive public key")
}

napi_value sn_crypto_scalarmult_ed25519 (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_scalarmult_ed25519)

  SN_ARGV_TYPEDARRAY(q, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(p, 2)

  SN_ASSERT_LENGTH(q_size, crypto_scalarmult_ed25519_BYTES, "q")
  SN_ASSERT_LENGTH(n_size, crypto_scalarmult_ed25519_SCALARBYTES, "n")
  SN_ASSERT_LENGTH(p_size, crypto_scalarmult_ed25519_BYTES, "p")

  SN_RETURN(crypto_scalarmult_ed25519(q_data, n_data, p_data), "failed to derive shared secret")
}

napi_value sn_crypto_core_ed25519_is_valid_point (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_core_ed25519_is_valid_point)

  SN_ARGV_TYPEDARRAY(p, 0)

  SN_ASSERT_LENGTH(p_size, crypto_core_ed25519_BYTES, "p")

  SN_RETURN_BOOLEAN_FROM_1(crypto_core_ed25519_is_valid_point(p_data))
}

napi_value sn_crypto_core_ed25519_from_uniform (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_core_ed25519_from_uniform)

  SN_ARGV_TYPEDARRAY(p, 0)
  SN_ARGV_TYPEDARRAY(r, 1)

  SN_ASSERT_LENGTH(p_size, crypto_core_ed25519_BYTES, "p")
  SN_ASSERT_LENGTH(r_size, crypto_core_ed25519_UNIFORMBYTES, "r")

  SN_RETURN(crypto_core_ed25519_from_uniform(p_data, r_data), "could not generate curve point from input")
}

napi_value sn_crypto_scalarmult_ed25519_base_noclamp (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_scalarmult_ed25519_base_noclamp)

  SN_ARGV_TYPEDARRAY(q, 0)
  SN_ARGV_TYPEDARRAY(n, 1)

  SN_ASSERT_LENGTH(q_size, crypto_scalarmult_ed25519_BYTES, "q")
  SN_ASSERT_LENGTH(n_size, crypto_scalarmult_ed25519_SCALARBYTES, "n")

  SN_RETURN(crypto_scalarmult_ed25519_base_noclamp(q_data, n_data), "failed to derive public key")
}

napi_value sn_crypto_scalarmult_ed25519_noclamp (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_scalarmult_ed25519_noclamp)

  SN_ARGV_TYPEDARRAY(q, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(p, 2)

  SN_ASSERT_LENGTH(q_size, crypto_scalarmult_ed25519_BYTES, "q")
  SN_ASSERT_LENGTH(n_size, crypto_scalarmult_ed25519_SCALARBYTES, "n")
  SN_ASSERT_LENGTH(p_size, crypto_scalarmult_ed25519_BYTES, "p")

  SN_RETURN(crypto_scalarmult_ed25519_noclamp(q_data, n_data, p_data), "failed to derive shared secret")
}

napi_value sn_crypto_core_ed25519_add (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_core_ed25519_add)

  SN_ARGV_TYPEDARRAY(r, 0)
  SN_ARGV_TYPEDARRAY(p, 1)
  SN_ARGV_TYPEDARRAY(q, 2)

  SN_ASSERT_LENGTH(r_size, crypto_core_ed25519_BYTES, "r")
  SN_ASSERT_LENGTH(p_size, crypto_core_ed25519_BYTES, "p")
  SN_ASSERT_LENGTH(q_size, crypto_core_ed25519_BYTES, "q")

  SN_RETURN(crypto_core_ed25519_add(r_data, p_data, q_data), "could not add curve points")
}

napi_value sn_crypto_core_ed25519_sub (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_core_ed25519_sub)

  SN_ARGV_TYPEDARRAY(r, 0)
  SN_ARGV_TYPEDARRAY(p, 1)
  SN_ARGV_TYPEDARRAY(q, 2)

  SN_ASSERT_LENGTH(r_size, crypto_core_ed25519_BYTES, "r")
  SN_ASSERT_LENGTH(p_size, crypto_core_ed25519_BYTES, "p")
  SN_ASSERT_LENGTH(q_size, crypto_core_ed25519_BYTES, "q")

  SN_RETURN(crypto_core_ed25519_sub(r_data, p_data, q_data), "could not add curve points")
}

napi_value sn_crypto_core_ed25519_scalar_random (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_core_ed25519_scalar_random)

  SN_ARGV_TYPEDARRAY(r, 0)

  SN_ASSERT_LENGTH(r_size, crypto_core_ed25519_SCALARBYTES, "r")

  crypto_core_ed25519_scalar_random(r_data);

  return NULL;
}

napi_value sn_crypto_core_ed25519_scalar_reduce (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_core_ed25519_scalar_reduce)

  SN_ARGV_TYPEDARRAY(r, 0)
  SN_ARGV_TYPEDARRAY(s, 1)

  SN_ASSERT_LENGTH(r_size, crypto_core_ed25519_SCALARBYTES, "r")
  SN_ASSERT_LENGTH(s_size, crypto_core_ed25519_NONREDUCEDSCALARBYTES, "s")

  crypto_core_ed25519_scalar_reduce(r_data, s_data);

  return NULL;
}

napi_value sn_crypto_core_ed25519_scalar_invert (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_core_ed25519_scalar_invert)

  SN_ARGV_TYPEDARRAY(recip, 0)
  SN_ARGV_TYPEDARRAY(s, 1)

  SN_ASSERT_LENGTH(recip_size, crypto_core_ed25519_SCALARBYTES, "recip")
  SN_ASSERT_LENGTH(s_size, crypto_core_ed25519_SCALARBYTES, "s")

  crypto_core_ed25519_scalar_invert(recip_data, s_data);

  return NULL;
}

napi_value sn_crypto_core_ed25519_scalar_negate (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_core_ed25519_scalar_negate)

  SN_ARGV_TYPEDARRAY(neg, 0)
  SN_ARGV_TYPEDARRAY(s, 1)

  SN_ASSERT_LENGTH(neg_size, crypto_core_ed25519_SCALARBYTES, "neg")
  SN_ASSERT_LENGTH(s_size, crypto_core_ed25519_SCALARBYTES, "s")

  crypto_core_ed25519_scalar_negate(neg_data, s_data);

  return NULL;
}

napi_value sn_crypto_core_ed25519_scalar_complement (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_core_ed25519_scalar_complement)

  SN_ARGV_TYPEDARRAY(comp, 0)
  SN_ARGV_TYPEDARRAY(s, 1)

  SN_ASSERT_LENGTH(comp_size, crypto_core_ed25519_SCALARBYTES, "comp")
  SN_ASSERT_LENGTH(s_size, crypto_core_ed25519_SCALARBYTES, "s")

  crypto_core_ed25519_scalar_complement(comp_data, s_data);

  return NULL;
}

napi_value sn_crypto_core_ed25519_scalar_add (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_core_ed25519_scalar_add)

  SN_ARGV_TYPEDARRAY(z, 0)
  SN_ARGV_TYPEDARRAY(x, 1)
  SN_ARGV_TYPEDARRAY(y, 2)

  SN_ASSERT_LENGTH(z_size, crypto_core_ed25519_SCALARBYTES, "z")
  SN_ASSERT_LENGTH(x_size, crypto_core_ed25519_SCALARBYTES, "x")
  SN_ASSERT_LENGTH(y_size, crypto_core_ed25519_SCALARBYTES, "y")

  crypto_core_ed25519_scalar_add(z_data, x_data, y_data);

  return NULL;
}

napi_value sn_crypto_core_ed25519_scalar_sub (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_core_ed25519_scalar_sub)

  SN_ARGV_TYPEDARRAY(z, 0)
  SN_ARGV_TYPEDARRAY(x, 1)
  SN_ARGV_TYPEDARRAY(y, 2)

  SN_ASSERT_LENGTH(z_size, crypto_core_ed25519_SCALARBYTES, "z")
  SN_ASSERT_LENGTH(x_size, crypto_core_ed25519_SCALARBYTES, "x")
  SN_ASSERT_LENGTH(y_size, crypto_core_ed25519_SCALARBYTES, "y")

  crypto_core_ed25519_scalar_sub(z_data, x_data, y_data);

  return NULL;
}

napi_value sn_crypto_shorthash (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_shorthash)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(in, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_ASSERT_LENGTH(out_size, crypto_shorthash_BYTES, "out")
  SN_ASSERT_LENGTH(k_size, crypto_shorthash_KEYBYTES, "k")

  SN_RETURN(crypto_shorthash(out_data, in_data, in_size, k_data), "could not compute hash")
}

napi_value sn_crypto_kdf_keygen (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_kdf_keygen)

  SN_ARGV_TYPEDARRAY(key, 0)

  SN_ASSERT_LENGTH(key_size, crypto_kdf_KEYBYTES, "key")

  crypto_kdf_keygen(key_data);

  return NULL;
}

napi_value sn_crypto_kdf_derive_from_key (napi_env env, napi_callback_info info) {
  SN_ARGV(4, crypto_kdf_derive_from_key)

  SN_ARGV_TYPEDARRAY(subkey, 0)
  SN_ARGV_UINT64(subkey_id, 1)
  SN_ARGV_TYPEDARRAY(ctx, 2)
  SN_ARGV_TYPEDARRAY(key, 3)

  SN_ASSERT_MIN_LENGTH(subkey_size, crypto_kdf_BYTES_MIN, "subkey")
  SN_ASSERT_MAX_LENGTH(subkey_size, crypto_kdf_BYTES_MAX, "subkey")
  SN_ASSERT_LENGTH(ctx_size, crypto_kdf_CONTEXTBYTES, "ctx")
  SN_ASSERT_LENGTH(key_size, crypto_kdf_KEYBYTES, "key")

  SN_RETURN(crypto_kdf_derive_from_key(subkey_data, subkey_size, subkey_id, ctx_data, key_data), "could not generate key")
}

napi_value sn_crypto_hash (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_hash_sha256)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(in, 1)

  SN_ASSERT_LENGTH(out_size, crypto_hash_BYTES, "out")

  SN_RETURN(crypto_hash(out_data, in_data, in_size), "could not compute hash")
}

napi_value sn_crypto_hash_sha256 (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_hash_sha256)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(in, 1)

  SN_ASSERT_LENGTH(out_size, crypto_hash_sha256_BYTES, "out")

  SN_RETURN(crypto_hash_sha256(out_data, in_data, in_size), "could not compute hash")
}

napi_value sn_crypto_hash_sha256_init (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_hash_sha256_init)

  SN_ARGV_BUFFER_CAST(crypto_hash_sha256_state *, state, 0)

  SN_THROWS(state_size != sizeof(crypto_hash_sha256_state), "state must be 'crypto_hash_sha256_STATEBYTES' bytes")

  SN_RETURN(crypto_hash_sha256_init(state), "failed to initialise sha256")
}

napi_value sn_crypto_hash_sha256_update(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_hash_sha256_update)

  SN_ARGV_BUFFER_CAST(crypto_hash_sha256_state *, state, 0)
  SN_ARGV_TYPEDARRAY(in, 1)

  SN_THROWS(state_size != sizeof(crypto_hash_sha256_state), "state must be 'crypto_hash_sha256_STATEBYTES' bytes")

  SN_RETURN(crypto_hash_sha256_update(state, in_data, in_size), "update failed")
}

napi_value sn_crypto_hash_sha256_final(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_hash_sha256_final)

  SN_ARGV_BUFFER_CAST(crypto_hash_sha256_state *, state, 0)
  SN_ARGV_TYPEDARRAY(out, 1)

  SN_THROWS(state_size != sizeof(crypto_hash_sha256_state), "state must be 'crypto_hash_sha256_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(out_size, crypto_hash_sha256_BYTES, "state")

  SN_RETURN(crypto_hash_sha256_final(state, out_data), "failed to finalise")
}

napi_value sn_crypto_hash_sha512 (napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_hash_sha512)

  SN_ARGV_TYPEDARRAY(out, 0)
  SN_ARGV_TYPEDARRAY(in, 1)

  SN_ASSERT_LENGTH(out_size, crypto_hash_sha512_BYTES, "out")

  SN_RETURN(crypto_hash_sha512(out_data, in_data, in_size), "could not compute hash")
}

napi_value sn_crypto_hash_sha512_init (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_hash_sha512_init)

  SN_ARGV_BUFFER_CAST(crypto_hash_sha512_state *, state, 0)

  SN_THROWS(state_size != sizeof(crypto_hash_sha512_state), "state must be 'crypto_hash_sha256_STATEBYTES' bytes")

  SN_RETURN(crypto_hash_sha512_init(state), "failed to initialise sha512")
}

napi_value sn_crypto_hash_sha512_update(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_hash_sha512_update)

  SN_ARGV_BUFFER_CAST(crypto_hash_sha512_state *, state, 0)
  SN_ARGV_TYPEDARRAY(in, 1)

  SN_THROWS(state_size != sizeof(crypto_hash_sha512_state), "state must be 'crypto_hash_sha256_STATEBYTES' bytes")

  SN_RETURN(crypto_hash_sha512_update(state, in_data, in_size), "update failed")
}

napi_value sn_crypto_hash_sha512_final(napi_env env, napi_callback_info info) {
  SN_ARGV(2, crypto_hash_sha512_final)

  SN_ARGV_BUFFER_CAST(crypto_hash_sha512_state *, state, 0)
  SN_ARGV_TYPEDARRAY(out, 1)

  SN_THROWS(state_size != sizeof(crypto_hash_sha512_state), "state must be 'crypto_hash_sha256_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(out_size, crypto_hash_sha512_BYTES, "out")

  SN_RETURN(crypto_hash_sha512_final(state, out_data), "failed to finalise hash")
}

napi_value sn_crypto_aead_xchacha20poly1305_ietf_keygen (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_aead_xchacha20poly1305_ietf_keygen)

  SN_ARGV_TYPEDARRAY(k, 0)

  SN_ASSERT_LENGTH(k_size, crypto_aead_xchacha20poly1305_ietf_KEYBYTES, "k")

  crypto_aead_xchacha20poly1305_ietf_keygen(k_data);
  return NULL;
}

napi_value sn_crypto_aead_xchacha20poly1305_ietf_encrypt (napi_env env, napi_callback_info info) {
  SN_ARGV(6, crypto_aead_xchacha20poly1305_ietf_encrypt)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 2)
  SN_ARGV_CHECK_NULL(nsec, 3)
  SN_ARGV_TYPEDARRAY(npub, 4)
  SN_ARGV_TYPEDARRAY(k, 5)

  SN_THROWS(!nsec_is_null, "nsec must always be set to null")

  SN_THROWS(c_size != m_size + crypto_aead_xchacha20poly1305_ietf_ABYTES, "c must 'm.byteLength + crypto_aead_xchacha20poly1305_ietf_ABYTES' bytes")
  SN_THROWS(c_size > 0xffffffff, "c.byteLength must be a 32bit integer")
  SN_ASSERT_LENGTH(npub_size, crypto_aead_xchacha20poly1305_ietf_NPUBBYTES, "npub")
  SN_ASSERT_LENGTH(k_size, crypto_aead_xchacha20poly1305_ietf_KEYBYTES, "k")

  unsigned long long clen;
  SN_CALL(crypto_aead_xchacha20poly1305_ietf_encrypt(c_data, &clen, m_data, m_size, ad_data, ad_size, NULL, npub_data, k_data), "could not encrypt data")

  napi_value result;
  SN_STATUS_THROWS(napi_create_uint32(env, (uint32_t) clen, &result), "")
  return result;
}

napi_value sn_crypto_aead_xchacha20poly1305_ietf_decrypt (napi_env env, napi_callback_info info) {
  SN_ARGV(6, crypto_aead_xchacha20poly1305_ietf_decrypt)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_CHECK_NULL(nsec, 1)
  SN_ARGV_TYPEDARRAY(c, 2)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 3)
  SN_ARGV_TYPEDARRAY(npub, 4)
  SN_ARGV_TYPEDARRAY(k, 5)

  SN_THROWS(!nsec_is_null, "nsec must always be set to null")

  SN_THROWS(m_size != c_size - crypto_aead_xchacha20poly1305_ietf_ABYTES, "m must 'c.byteLength - crypto_aead_xchacha20poly1305_ietf_ABYTES' bytes")
  SN_ASSERT_LENGTH(npub_size, crypto_aead_xchacha20poly1305_ietf_NPUBBYTES, "npub")
  SN_ASSERT_LENGTH(k_size, crypto_aead_xchacha20poly1305_ietf_KEYBYTES, "k")
  SN_THROWS(m_size > 0xffffffff, "m.byteLength must be a 32bit integer")

  unsigned long long mlen;
  SN_CALL(crypto_aead_xchacha20poly1305_ietf_decrypt(m_data, &mlen, NULL, c_data, c_size, ad_data, ad_size, npub_data, k_data), "could not verify data")

  napi_value result;
  SN_STATUS_THROWS(napi_create_uint32(env, (uint32_t) mlen, &result), "")
  return result;
}

napi_value sn_crypto_aead_xchacha20poly1305_ietf_encrypt_detached (napi_env env, napi_callback_info info) {
  SN_ARGV(7, crypto_aead_xchacha20poly1305_ietf_encrypt_detached)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(mac, 1)
  SN_ARGV_TYPEDARRAY(m, 2)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 3)
  SN_ARGV_CHECK_NULL(nsec, 4)
  SN_ARGV_TYPEDARRAY(npub, 5)
  SN_ARGV_TYPEDARRAY(k, 6)

  SN_THROWS(!nsec_is_null, "nsec must always be set to null")

  SN_THROWS(c_size != m_size, "c must be 'm.byteLength' bytes")
  SN_ASSERT_LENGTH(mac_size, crypto_aead_xchacha20poly1305_ietf_ABYTES, "mac")
  SN_ASSERT_LENGTH(npub_size, crypto_aead_xchacha20poly1305_ietf_NPUBBYTES, "npub")
  SN_ASSERT_LENGTH(k_size, crypto_aead_xchacha20poly1305_ietf_KEYBYTES, "k")

  unsigned long long maclen;
  SN_CALL(crypto_aead_xchacha20poly1305_ietf_encrypt_detached(c_data, mac_data, &maclen, m_data, m_size, ad_data, ad_size, NULL, npub_data, k_data), "could not encrypt data")

  napi_value result;
  SN_STATUS_THROWS(napi_create_uint32(env, (uint32_t) maclen, &result), "")
  return result;
}

napi_value sn_crypto_aead_xchacha20poly1305_ietf_decrypt_detached (napi_env env, napi_callback_info info) {
  SN_ARGV(7, crypto_aead_xchacha20poly1305_ietf_decrypt_detached)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_CHECK_NULL(nsec, 1)
  SN_ARGV_TYPEDARRAY(c, 2)
  SN_ARGV_TYPEDARRAY(mac, 3)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 4)
  SN_ARGV_TYPEDARRAY(npub, 5)
  SN_ARGV_TYPEDARRAY(k, 6)

  SN_THROWS(!nsec_is_null, "nsec must always be set to null")

  SN_THROWS(m_size != c_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(mac_size, crypto_aead_xchacha20poly1305_ietf_ABYTES, "mac")
  SN_ASSERT_LENGTH(npub_size, crypto_aead_xchacha20poly1305_ietf_NPUBBYTES, "npub")
  SN_ASSERT_LENGTH(k_size, crypto_aead_xchacha20poly1305_ietf_KEYBYTES, "k")

  SN_RETURN(crypto_aead_xchacha20poly1305_ietf_decrypt_detached(m_data, NULL, c_data, c_size, mac_data, ad_data, ad_size, npub_data, k_data), "could not verify data")
}

napi_value sn_crypto_aead_chacha20poly1305_ietf_keygen (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_aead_chacha20poly1305_ietf_keygen)

  SN_ARGV_TYPEDARRAY(k, 0)

  SN_ASSERT_LENGTH(k_size, crypto_aead_chacha20poly1305_ietf_KEYBYTES, "k")

  crypto_aead_chacha20poly1305_ietf_keygen(k_data);
  return NULL;
}

napi_value sn_crypto_aead_chacha20poly1305_ietf_encrypt (napi_env env, napi_callback_info info) {
  SN_ARGV(6, crypto_aead_chacha20poly1305_ietf_encrypt)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 2)
  SN_ARGV_CHECK_NULL(nsec, 3)
  SN_ARGV_TYPEDARRAY(npub, 4)
  SN_ARGV_TYPEDARRAY(k, 5)

  SN_THROWS(!nsec_is_null, "nsec must always be set to null")

  SN_THROWS(c_size != m_size + crypto_aead_chacha20poly1305_ietf_ABYTES, "c must 'm.byteLength + crypto_aead_chacha20poly1305_ietf_ABYTES' bytes")
  SN_THROWS(c_size > 0xffffffff, "c.byteLength must be a 32bit integer")
  SN_ASSERT_LENGTH(npub_size, crypto_aead_chacha20poly1305_ietf_NPUBBYTES, "npub")
  SN_ASSERT_LENGTH(k_size, crypto_aead_chacha20poly1305_ietf_KEYBYTES, "k")

  unsigned long long clen;
  SN_CALL(crypto_aead_chacha20poly1305_ietf_encrypt(c_data, &clen, m_data, m_size, ad_data, ad_size, NULL, npub_data, k_data), "could not encrypt data")

  napi_value result;
  SN_STATUS_THROWS(napi_create_uint32(env, (uint32_t) clen, &result), "")
  return result;
}

napi_value sn_crypto_aead_chacha20poly1305_ietf_decrypt (napi_env env, napi_callback_info info) {
  SN_ARGV(6, crypto_aead_chacha20poly1305_ietf_decrypt)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_CHECK_NULL(nsec, 1)
  SN_ARGV_TYPEDARRAY(c, 2)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 3)
  SN_ARGV_TYPEDARRAY(npub, 4)
  SN_ARGV_TYPEDARRAY(k, 5)

  SN_THROWS(!nsec_is_null, "nsec must always be set to null")

  SN_THROWS(m_size != c_size - crypto_aead_chacha20poly1305_ietf_ABYTES, "m must 'c.byteLength - crypto_aead_chacha20poly1305_ietf_ABYTES' bytes")
  SN_ASSERT_LENGTH(npub_size, crypto_aead_chacha20poly1305_ietf_NPUBBYTES, "npub")
  SN_ASSERT_LENGTH(k_size, crypto_aead_chacha20poly1305_ietf_KEYBYTES, "k")
  SN_THROWS(m_size > 0xffffffff, "m.byteLength must be a 32bit integer")

  unsigned long long mlen;
  SN_CALL(crypto_aead_chacha20poly1305_ietf_decrypt(m_data, &mlen, NULL, c_data, c_size, ad_data, ad_size, npub_data, k_data), "could not verify data")

  napi_value result;
  SN_STATUS_THROWS(napi_create_uint32(env, (uint32_t) mlen, &result), "")
  return result;
}

napi_value sn_crypto_aead_chacha20poly1305_ietf_encrypt_detached (napi_env env, napi_callback_info info) {
  SN_ARGV(7, crypto_aead_chacha20poly1305_ietf_encrypt_detached)

  SN_ARGV_TYPEDARRAY(c, 0)
  SN_ARGV_TYPEDARRAY(mac, 1)
  SN_ARGV_TYPEDARRAY(m, 2)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 3)
  SN_ARGV_CHECK_NULL(nsec, 4)
  SN_ARGV_TYPEDARRAY(npub, 5)
  SN_ARGV_TYPEDARRAY(k, 6)

  SN_THROWS(!nsec_is_null, "nsec must always be set to null")

  SN_THROWS(c_size != m_size, "c must be 'm.byteLength' bytes")
  SN_ASSERT_LENGTH(mac_size, crypto_aead_chacha20poly1305_ietf_ABYTES, "mac")
  SN_ASSERT_LENGTH(npub_size, crypto_aead_chacha20poly1305_ietf_NPUBBYTES, "npub")
  SN_ASSERT_LENGTH(k_size, crypto_aead_chacha20poly1305_ietf_KEYBYTES, "k")

  unsigned long long maclen;
  SN_CALL(crypto_aead_chacha20poly1305_ietf_encrypt_detached(c_data, mac_data, &maclen, m_data, m_size, ad_data, ad_size, NULL, npub_data, k_data), "could not encrypt data")

  napi_value result;
  SN_STATUS_THROWS(napi_create_uint32(env, (uint32_t) maclen, &result), "")
  return result;
}

napi_value sn_crypto_aead_chacha20poly1305_ietf_decrypt_detached (napi_env env, napi_callback_info info) {
  SN_ARGV(7, crypto_aead_chacha20poly1305_ietf_decrypt_detached)

  SN_ARGV_TYPEDARRAY(m, 0)
  SN_ARGV_CHECK_NULL(nsec, 1)
  SN_ARGV_TYPEDARRAY(c, 2)
  SN_ARGV_TYPEDARRAY(mac, 3)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 4)
  SN_ARGV_TYPEDARRAY(npub, 5)
  SN_ARGV_TYPEDARRAY(k, 6)

  SN_THROWS(!nsec_is_null, "nsec must always be set to null")

  SN_THROWS(m_size != c_size, "m must be 'c.byteLength' bytes")
  SN_ASSERT_LENGTH(mac_size, crypto_aead_chacha20poly1305_ietf_ABYTES, "mac")
  SN_ASSERT_LENGTH(npub_size, crypto_aead_chacha20poly1305_ietf_NPUBBYTES, "npub")
  SN_ASSERT_LENGTH(k_size, crypto_aead_chacha20poly1305_ietf_KEYBYTES, "k")

  SN_RETURN(crypto_aead_chacha20poly1305_ietf_decrypt_detached(m_data, NULL, c_data, c_size, mac_data, ad_data, ad_size, npub_data, k_data), "could not verify data")
}

napi_value sn_crypto_secretstream_xchacha20poly1305_keygen (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_secretstream_xchacha20poly1305_keygen)

  SN_ARGV_TYPEDARRAY(k, 0)

  SN_ASSERT_LENGTH(k_size, crypto_secretstream_xchacha20poly1305_KEYBYTES, "k")

  crypto_secretstream_xchacha20poly1305_keygen(k_data);

  return NULL;
}

napi_value sn_crypto_secretstream_xchacha20poly1305_init_push (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_secretstream_xchacha20poly1305_init_push)

  SN_ARGV_BUFFER_CAST(crypto_secretstream_xchacha20poly1305_state *, state, 0)
  SN_ARGV_TYPEDARRAY(header, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_THROWS(state_size != sizeof(crypto_secretstream_xchacha20poly1305_state), "state must be 'crypto_secretstream_xchacha20poly1305_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(header_size, crypto_secretstream_xchacha20poly1305_HEADERBYTES, "header")
  SN_ASSERT_LENGTH(k_size, crypto_secretstream_xchacha20poly1305_KEYBYTES, "k")

  SN_RETURN(crypto_secretstream_xchacha20poly1305_init_push(state, header_data, k_data), "initial push failed")
}

napi_value sn_crypto_secretstream_xchacha20poly1305_push (napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_secretstream_xchacha20poly1305_push)

  SN_ARGV_BUFFER_CAST(crypto_secretstream_xchacha20poly1305_state *, state, 0)
  SN_ARGV_TYPEDARRAY(c, 1)
  SN_ARGV_TYPEDARRAY(m, 2)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 3)
  SN_ARGV_UINT8(tag, 4)

  SN_THROWS(state_size != sizeof(crypto_secretstream_xchacha20poly1305_state), "state must be state must be 'crypto_secretstream_xchacha20poly1305_STATEBYTES' bytes")
  SN_ASSERT_MAX_LENGTH(m_size, crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX, "m")
  SN_THROWS(c_size != m_size + crypto_secretstream_xchacha20poly1305_ABYTES, "c must be 'm.byteLength + crypto_secretstream_xchacha20poly1305_ABYTES' bytes")
  SN_THROWS(c_size > 0xffffffff, "c.byteLength must be a 32bit integer")

  unsigned long long clen;
  SN_CALL(crypto_secretstream_xchacha20poly1305_push(state, c_data, &clen, m_data, m_size, ad_data, ad_size, tag), "push failed")

  napi_value result;
  SN_STATUS_THROWS(napi_create_uint32(env, (uint32_t) clen, &result), "")
  return result;
}

napi_value sn_crypto_secretstream_xchacha20poly1305_init_pull (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_secretstream_xchacha20poly1305_init_pull)

  SN_ARGV_BUFFER_CAST(crypto_secretstream_xchacha20poly1305_state *, state, 0)
  SN_ARGV_TYPEDARRAY(header, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_THROWS(state_size != sizeof(crypto_secretstream_xchacha20poly1305_state), "state must be 'crypto_secretstream_xchacha20poly1305_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(header_size, crypto_secretstream_xchacha20poly1305_HEADERBYTES, "header")
  SN_ASSERT_LENGTH(k_size, crypto_secretstream_xchacha20poly1305_KEYBYTES, "k")

  SN_RETURN(crypto_secretstream_xchacha20poly1305_init_pull(state, header_data, k_data), "initial pull failed")
}

napi_value sn_crypto_secretstream_xchacha20poly1305_pull (napi_env env, napi_callback_info info) {
  SN_ARGV(5, crypto_secretstream_xchacha20poly1305_pull)

  SN_ARGV_BUFFER_CAST(crypto_secretstream_xchacha20poly1305_state *, state, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(tag, 2)
  SN_ARGV_TYPEDARRAY(c, 3)
  SN_ARGV_OPTS_TYPEDARRAY(ad, 4)

  SN_THROWS(state_size != sizeof(crypto_secretstream_xchacha20poly1305_state), "state must be 'crypto_secretstream_xchacha20poly1305_STATEBYTES' bytes")
  SN_ASSERT_MIN_LENGTH(c_size, crypto_secretstream_xchacha20poly1305_ABYTES, "c")
  SN_ASSERT_LENGTH(tag_size, 1, "tag")
  SN_THROWS(m_size != c_size - crypto_secretstream_xchacha20poly1305_ABYTES, "m must be 'c.byteLength - crypto_secretstream_xchacha20poly1305_ABYTES")
  SN_THROWS(m_size > 0xffffffff, "m.byteLength must be a 32bit integer")

  unsigned long long mlen;
  SN_CALL(crypto_secretstream_xchacha20poly1305_pull(state, m_data, &mlen, tag_data, c_data, c_size, ad_data, ad_size), "pull failed")

  napi_value result;
  SN_STATUS_THROWS(napi_create_uint32(env, (uint32_t) mlen, &result), "")
  return result;
}

napi_value sn_crypto_secretstream_xchacha20poly1305_rekey (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_secretstream_xchacha20poly1305_rekey)

  SN_ARGV_BUFFER_CAST(crypto_secretstream_xchacha20poly1305_state *, state, 0)

  SN_THROWS(state_size != sizeof(crypto_secretstream_xchacha20poly1305_state), "state must be 'crypto_secretstream_xchacha20poly1305_STATEBYTES' bytes")

  crypto_secretstream_xchacha20poly1305_rekey(state);

  return NULL;
}

typedef struct sn_async_task_t {
  uv_work_t task;

  enum {
    sn_async_task_promise,
    sn_async_task_callback
  } type;

  void *req;
  int code;

  napi_deferred deferred;
  napi_ref cb;
} sn_async_task_t;

typedef struct sn_async_pwhash_request {
  napi_env env;
  napi_ref out_ref;
  unsigned char *out_data;
  size_t out_size;
  napi_ref pwd_ref;
  const char *pwd_data;
  size_t pwd_size;
  napi_ref salt_ref;
  unsigned char *salt;
  uint32_t opslimit;
  uint32_t memlimit;
  uint32_t alg;
} sn_async_pwhash_request;

static void async_pwhash_execute (uv_work_t *uv_req) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_request *req = (sn_async_pwhash_request *) task->req;
  task->code = crypto_pwhash(req->out_data,
                             req->out_size,
                             req->pwd_data,
                             req->pwd_size,
                             req->salt,
                             req->opslimit,
                             req->memlimit,
                             req->alg);
}

static void async_pwhash_complete (uv_work_t *uv_req, int status) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_request *req = (sn_async_pwhash_request *) task->req;

  napi_handle_scope scope;
  napi_open_handle_scope(req->env, &scope);

  napi_value global;
  napi_get_global(req->env, &global);

  SN_ASYNC_COMPLETE("failed to compute password hash")

  napi_close_handle_scope(req->env, scope);

  napi_delete_reference(req->env, req->out_ref);
  napi_delete_reference(req->env, req->pwd_ref);
  napi_delete_reference(req->env, req->salt_ref);

  free(req);
  free(task);
}

napi_value sn_crypto_pwhash_async (napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(6, 7, crypto_pwhash_async)

  SN_ARGV_BUFFER_CAST(unsigned char *, out, 0)
  SN_ARGV_BUFFER_CAST(char *, pwd, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, salt, 2)
  SN_ARGV_UINT64(opslimit, 3)
  SN_ARGV_UINT64(memlimit, 4)
  SN_ARGV_UINT8(alg, 5)

  SN_ASSERT_MIN_LENGTH(out_size, crypto_pwhash_BYTES_MIN, "out")
  SN_ASSERT_MAX_LENGTH(out_size, crypto_pwhash_BYTES_MAX, "out")
  SN_ASSERT_LENGTH(salt_size, crypto_pwhash_SALTBYTES, "salt")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_MEMLIMIT_MAX, "memlimit")
  SN_THROWS(alg < 1 || alg > 2, "alg must be either Argon2i 1.3 or Argon2id 1.3")

  sn_async_pwhash_request *req = (sn_async_pwhash_request *) malloc(sizeof(sn_async_pwhash_request));

  req->env = env;
  req->out_data = out;
  req->out_size = out_size;
  req->pwd_data = pwd;
  req->pwd_size = pwd_size;
  req->salt = salt;
  req->opslimit = opslimit;
  req->memlimit = memlimit;
  req->alg = alg;

  sn_async_task_t *task = (sn_async_task_t *) malloc(sizeof(sn_async_task_t));
  SN_ASYNC_TASK(6)

  SN_STATUS_THROWS(napi_create_reference(env, out_argv, 1, &req->out_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, pwd_argv, 1, &req->pwd_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, salt_argv, 1, &req->salt_ref), "")

  SN_QUEUE_TASK(task, async_pwhash_execute, async_pwhash_complete)

  return promise;
}

typedef struct sn_async_pwhash_str_request {
  uv_work_t task;
  napi_env env;
  napi_ref out_ref;
  char *out_data;
  napi_ref pwd_ref;
  const char *pwd_data;
  size_t pwd_size;
  uint32_t opslimit;
  uint32_t memlimit;
} sn_async_pwhash_str_request;

static void async_pwhash_str_execute (uv_work_t *uv_req) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_str_request *req = (sn_async_pwhash_str_request *) task->req;
  task->code = crypto_pwhash_str(req->out_data,
                                 req->pwd_data,
                                 req->pwd_size,
                                 req->opslimit,
                                 req->memlimit);
}

static void async_pwhash_str_complete (uv_work_t *uv_req, int status) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_str_request *req = (sn_async_pwhash_str_request *) task->req;

  napi_handle_scope scope;
  napi_open_handle_scope(req->env, &scope);

  napi_value global;
  napi_get_global(req->env, &global);

  SN_ASYNC_COMPLETE("failed to compute password hash")

  napi_close_handle_scope(req->env, scope);

  napi_delete_reference(req->env, req->out_ref);
  napi_delete_reference(req->env, req->pwd_ref);

  free(req);
  free(task);
}

napi_value sn_crypto_pwhash_str_async (napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(4, 5, crypto_pwhash_str_async)

  SN_ARGV_BUFFER_CAST(char *, out, 0)
  SN_ARGV_BUFFER_CAST(char *, pwd, 1)
  SN_ARGV_UINT64(opslimit, 2)
  SN_ARGV_UINT64(memlimit, 3)

  SN_ASSERT_LENGTH(out_size, crypto_pwhash_STRBYTES, "out")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_MEMLIMIT_MAX, "memlimit")

  sn_async_pwhash_str_request *req = (sn_async_pwhash_str_request *) malloc(sizeof(sn_async_pwhash_str_request));
  req->env = env;
  req->out_data = out;
  req->pwd_data = pwd;
  req->pwd_size = pwd_size;
  req->opslimit = opslimit;
  req->memlimit = memlimit;

  sn_async_task_t *task = (sn_async_task_t *) malloc(sizeof(sn_async_task_t));
  SN_ASYNC_TASK(4)

  SN_STATUS_THROWS(napi_create_reference(env, out_argv, 1, &req->out_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, pwd_argv, 1, &req->pwd_ref), "")

  SN_QUEUE_TASK(task, async_pwhash_str_execute, async_pwhash_str_complete)

  return promise;
}

typedef struct sn_async_pwhash_str_verify_request {
  uv_work_t task;
  napi_env env;
  napi_ref str_ref;
  char *str_data;
  napi_ref pwd_ref;
  const char *pwd_data;
  size_t pwd_size;
} sn_async_pwhash_str_verify_request;

static void async_pwhash_str_verify_execute (uv_work_t *uv_req) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_str_verify_request *req = (sn_async_pwhash_str_verify_request *) task->req;
  task->code = crypto_pwhash_str_verify(req->str_data, req->pwd_data, req->pwd_size);
}

static void async_pwhash_str_verify_complete (uv_work_t *uv_req, int status) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_str_verify_request *req = (sn_async_pwhash_str_verify_request *) task->req;

  napi_handle_scope scope;
  napi_open_handle_scope(req->env, &scope);

  napi_value global;
  napi_get_global(req->env, &global);

  napi_value argv[2];

  // Due to the way that crypto_pwhash_str_verify signals error different
  // from a verification mismatch, we will count all errors as mismatch.
  // The other possible error is wrong argument sizes, which is protected
  // by macros above
  napi_get_null(req->env, &argv[0]);
  napi_get_boolean(req->env, task->code == 0, &argv[1]);

  switch (task->type) {
  case sn_async_task_promise: {
    napi_resolve_deferred(req->env, task->deferred, argv[1]);
    task->deferred = NULL;
    break;
  }

  case sn_async_task_callback: {
    napi_value callback;
    napi_get_reference_value(req->env, task->cb, &callback);

    napi_value return_val;
    SN_CALL_FUNCTION(req->env, global, callback, 2, argv, &return_val)
    break;
  }
  }

  napi_close_handle_scope(req->env, scope);

  napi_delete_reference(req->env, req->str_ref);
  napi_delete_reference(req->env, req->pwd_ref);

  free(req);
  free(task);
}

napi_value sn_crypto_pwhash_str_verify_async (napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(2, 3, crypto_pwhash_str_async)

  SN_ARGV_BUFFER_CAST(char *, str, 0)
  SN_ARGV_BUFFER_CAST(char *, pwd, 1)

  SN_ASSERT_LENGTH(str_size, crypto_pwhash_STRBYTES, "str")

  sn_async_pwhash_str_verify_request *req = (sn_async_pwhash_str_verify_request *) malloc(sizeof(sn_async_pwhash_str_verify_request));
  req->env = env;
  req->str_data = str;
  req->pwd_data = pwd;
  req->pwd_size = pwd_size;

  sn_async_task_t *task = (sn_async_task_t *) malloc(sizeof(sn_async_task_t));
  SN_ASYNC_TASK(2)

  SN_STATUS_THROWS(napi_create_reference(env, str_argv, 1, &req->str_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, pwd_argv, 1, &req->pwd_ref), "")

  SN_QUEUE_TASK(task, async_pwhash_str_verify_execute, async_pwhash_str_verify_complete)

  return promise;
}

typedef struct sn_async_pwhash_scryptsalsa208sha256_request {
  uv_work_t task;
  napi_env env;
  napi_ref out_ref;
  unsigned char *out_data;
  size_t out_size;
  napi_ref pwd_ref;
  const char *pwd_data;
  size_t pwd_size;
  napi_ref salt_ref;
  unsigned char *salt;
  uint32_t opslimit;
  uint32_t memlimit;
} sn_async_pwhash_scryptsalsa208sha256_request;

static void async_pwhash_scryptsalsa208sha256_execute (uv_work_t *uv_req) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_scryptsalsa208sha256_request *req = (sn_async_pwhash_scryptsalsa208sha256_request *) task->req;
  task->code = crypto_pwhash_scryptsalsa208sha256(req->out_data,
                                                  req->out_size,
                                                  req-> pwd_data,
                                                  req->pwd_size,
                                                  req->salt,
                                                  req->opslimit,
                                                  req->memlimit);
}

static void async_pwhash_scryptsalsa208sha256_complete (uv_work_t *uv_req, int status) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_scryptsalsa208sha256_request *req = (sn_async_pwhash_scryptsalsa208sha256_request *) task->req;

  napi_handle_scope scope;
  napi_open_handle_scope(req->env, &scope);

  napi_value global;
  napi_get_global(req->env, &global);

  SN_ASYNC_COMPLETE("failed to compute password hash")

  napi_close_handle_scope(req->env, scope);

  napi_delete_reference(req->env, req->out_ref);
  napi_delete_reference(req->env, req->pwd_ref);
  napi_delete_reference(req->env, req->salt_ref);

  free(req);
  free(task);
}

napi_value sn_crypto_pwhash_scryptsalsa208sha256_async (napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(5, 6, crypto_pwhash_scryptsalsa208sha256_async)

  SN_ARGV_BUFFER_CAST(unsigned char *, out, 0)
  SN_ARGV_BUFFER_CAST(char *, pwd, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, salt, 2)
  SN_ARGV_UINT64(opslimit, 3)
  SN_ARGV_UINT64(memlimit, 4)

  SN_ASSERT_MIN_LENGTH(out_size, crypto_pwhash_scryptsalsa208sha256_BYTES_MIN, "out")
  SN_ASSERT_MAX_LENGTH(out_size, crypto_pwhash_scryptsalsa208sha256_BYTES_MAX, "out")
  SN_ASSERT_LENGTH(salt_size, crypto_pwhash_scryptsalsa208sha256_SALTBYTES, "salt")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX, "memlimit")

  sn_async_pwhash_scryptsalsa208sha256_request *req = (sn_async_pwhash_scryptsalsa208sha256_request *) malloc(sizeof(sn_async_pwhash_scryptsalsa208sha256_request));
  req->env = env;
  req->out_data = out;
  req->out_size = out_size;
  req->pwd_data = pwd;
  req->pwd_size = pwd_size;
  req-> salt = salt;
  req->opslimit = opslimit;
  req->memlimit = memlimit;

  sn_async_task_t *task = (sn_async_task_t *) malloc(sizeof(sn_async_task_t));
  SN_ASYNC_TASK(5)

  SN_STATUS_THROWS(napi_create_reference(env, out_argv, 1, &req->out_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, pwd_argv, 1, &req->pwd_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, salt_argv, 1, &req->salt_ref), "")

  SN_QUEUE_TASK(task, async_pwhash_scryptsalsa208sha256_execute, async_pwhash_scryptsalsa208sha256_complete)

  return promise;
}

typedef struct sn_async_pwhash_scryptsalsa208sha256_str_request {
  uv_work_t task;
  napi_env env;
  napi_ref out_ref;
  char *out_data;
  napi_ref pwd_ref;
  const char *pwd_data;
  size_t pwd_size;
  uint32_t opslimit;
  uint32_t memlimit;
} sn_async_pwhash_scryptsalsa208sha256_str_request;

static void async_pwhash_scryptsalsa208sha256_str_execute (uv_work_t *uv_req) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_scryptsalsa208sha256_str_request *req = (sn_async_pwhash_scryptsalsa208sha256_str_request *) task->req;
  task->code = crypto_pwhash_scryptsalsa208sha256_str(req->out_data,
                                                      req->pwd_data,
                                                      req->pwd_size,
                                                      req->opslimit,
                                                      req->memlimit);
}

static void async_pwhash_scryptsalsa208sha256_str_complete (uv_work_t *uv_req, int status) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_scryptsalsa208sha256_str_request *req = (sn_async_pwhash_scryptsalsa208sha256_str_request *) task->req;

  napi_handle_scope scope;
  napi_open_handle_scope(req->env, &scope);

  napi_value global;
  napi_get_global(req->env, &global);

  SN_ASYNC_COMPLETE("failed to compute password hash")

  napi_close_handle_scope(req->env, scope);

  napi_delete_reference(req->env, req->out_ref);
  napi_delete_reference(req->env, req->pwd_ref);

  free(req);
  free(task);
}

napi_value sn_crypto_pwhash_scryptsalsa208sha256_str_async (napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(4, 5, crypto_pwhash_scryptsalsa208sha256_str_async)

  SN_ARGV_BUFFER_CAST(char *, out, 0)
  SN_ARGV_BUFFER_CAST(char *, pwd, 1)
  SN_ARGV_UINT64(opslimit, 2)
  SN_ARGV_UINT64(memlimit, 3)

  SN_ASSERT_LENGTH(out_size, crypto_pwhash_scryptsalsa208sha256_STRBYTES, "out")
  SN_ASSERT_MIN_LENGTH(opslimit, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN, "opslimit")
  SN_ASSERT_MAX_LENGTH(opslimit, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX, "opslimit")
  SN_ASSERT_MIN_LENGTH(memlimit, crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN, "memlimit")
  SN_ASSERT_MAX_LENGTH(memlimit, (int64_t) crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX, "memlimit")

  sn_async_pwhash_scryptsalsa208sha256_str_request *req = (sn_async_pwhash_scryptsalsa208sha256_str_request *) malloc(sizeof(sn_async_pwhash_scryptsalsa208sha256_str_request));
  req->env = env;
  req->out_data = out;
  req->pwd_data = pwd;
  req->pwd_size = pwd_size;
  req->opslimit = opslimit;
  req->memlimit = memlimit;

  sn_async_task_t *task = (sn_async_task_t *) malloc(sizeof(sn_async_task_t));
  SN_ASYNC_TASK(4)

  SN_STATUS_THROWS(napi_create_reference(env, out_argv, 1, &req->out_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, pwd_argv, 1, &req->pwd_ref), "")

  SN_QUEUE_TASK(task, async_pwhash_scryptsalsa208sha256_str_execute, async_pwhash_scryptsalsa208sha256_str_complete)

  return promise;
}

typedef struct sn_async_pwhash_scryptsalsa208sha256_str_verify_request {
  uv_work_t task;
  napi_env env;
  napi_ref str_ref;
  char *str_data;
  napi_ref pwd_ref;
  const char *pwd_data;
  size_t pwd_size;
} sn_async_pwhash_scryptsalsa208sha256_str_verify_request;

static void async_pwhash_scryptsalsa208sha256_str_verify_execute (uv_work_t *uv_req) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_scryptsalsa208sha256_str_verify_request *req = (sn_async_pwhash_scryptsalsa208sha256_str_verify_request *) task->req;
  task->code = crypto_pwhash_scryptsalsa208sha256_str_verify(req->str_data, req->pwd_data, req->pwd_size);
}

static void async_pwhash_scryptsalsa208sha256_str_verify_complete (uv_work_t *uv_req, int status) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pwhash_scryptsalsa208sha256_str_verify_request *req = (sn_async_pwhash_scryptsalsa208sha256_str_verify_request *) task->req;

  napi_handle_scope scope;
  napi_open_handle_scope(req->env, &scope);

  napi_value global;
  napi_get_global(req->env, &global);

  napi_value argv[2];

  // Due to the way that crypto_pwhash_scryptsalsa208sha256_str_verify
  // signal serror different from a verification mismatch, we will count
  // all errors as mismatch. The other possible error is wrong argument
  // sizes, which is protected by macros above
  napi_get_null(req->env, &argv[0]);
  napi_get_boolean(req->env, task->code == 0, &argv[1]);

  switch (task->type) {
  case sn_async_task_promise: {
    napi_resolve_deferred(req->env, task->deferred, argv[1]);
    task->deferred = NULL;
    break;
  }

  case sn_async_task_callback: {
    napi_value callback;
    napi_get_reference_value(req->env, task->cb, &callback);

    napi_value return_val;
    SN_CALL_FUNCTION(req->env, global, callback, 2, argv, &return_val)
    break;
  }
  }

  napi_close_handle_scope(req->env, scope);

  napi_delete_reference(req->env, req->str_ref);
  napi_delete_reference(req->env, req->pwd_ref);

  free(req);
  free(task);
}

napi_value sn_crypto_pwhash_scryptsalsa208sha256_str_verify_async (napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(2, 3, crypto_pwhash_scryptsalsa208sha256_str_async)

  SN_ARGV_BUFFER_CAST(char *, str, 0)
  SN_ARGV_BUFFER_CAST(char *, pwd, 1)

  SN_ASSERT_LENGTH(str_size, crypto_pwhash_scryptsalsa208sha256_STRBYTES, "str")

  sn_async_pwhash_scryptsalsa208sha256_str_verify_request *req = (sn_async_pwhash_scryptsalsa208sha256_str_verify_request *) malloc(sizeof(sn_async_pwhash_scryptsalsa208sha256_str_verify_request));
  req->env = env;
  req->str_data = str;
  req->pwd_data = pwd;
  req->pwd_size = pwd_size;

  sn_async_task_t *task = (sn_async_task_t *) malloc(sizeof(sn_async_task_t));
  SN_ASYNC_TASK(2)

  SN_STATUS_THROWS(napi_create_reference(env, str_argv, 1, &req->str_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, pwd_argv, 1, &req->pwd_ref), "")

  SN_QUEUE_TASK(task, async_pwhash_scryptsalsa208sha256_str_verify_execute, async_pwhash_scryptsalsa208sha256_str_verify_complete)

  return promise;
}

typedef struct sn_crypto_stream_xor_state {
  unsigned char n[crypto_stream_NONCEBYTES];
  unsigned char k[crypto_stream_KEYBYTES];
  unsigned char next_block[64];
  int remainder;
  uint64_t block_counter;
} sn_crypto_stream_xor_state;

napi_value sn_crypto_stream_xor_wrap_init (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_xor_instance_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_xor_state *, state, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_xor_state), "state must be 'sn_crypto_stream_xor_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_KEYBYTES, "k")

  state->remainder = 0;
  state->block_counter = 0;
  memcpy(state->n, n_data, crypto_stream_NONCEBYTES);
  memcpy(state->k, k_data, crypto_stream_KEYBYTES);

  return NULL;
}

napi_value sn_crypto_stream_xor_wrap_update (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_xor_instance_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_xor_state *, state, 0)
  SN_ARGV_BUFFER_CAST(unsigned char *, c, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, m, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_xor_state), "state must be 'sn_crypto_stream_xor_STATEBYTES' bytes")
  SN_THROWS(c_size != m_size, "c must be 'm.byteLength' bytes")

  unsigned char *next_block = state->next_block;

  if (state->remainder) {
    uint64_t offset = 0;
    int rem = state->remainder;

    while (rem < 64 && offset < m_size) {
      c[offset] = next_block[rem]  ^ m[offset];
      offset++;
      rem++;
    }

    c += offset;
    m += offset;
    m_size -= offset;
    state->remainder = rem == 64 ? 0 : rem;

    if (!m_size) return NULL;
  }

  state->remainder = m_size & 63;
  m_size -= state->remainder;
  crypto_stream_xsalsa20_xor_ic(c, m, m_size, state->n, state->block_counter, state->k);
  state->block_counter += m_size / 64;

  if (state->remainder) {
    sodium_memzero(next_block + state->remainder, 64 - state->remainder);
    memcpy(next_block, m + m_size, state->remainder);

    crypto_stream_xsalsa20_xor_ic(next_block, next_block, 64, state->n, state->block_counter, state->k);
    memcpy(c + m_size, next_block, state->remainder);

    state->block_counter++;
  }

  return NULL;
}

napi_value sn_crypto_stream_xor_wrap_final (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_stream_xor_instance_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_xor_state *, state, 0)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_xor_state), "state must be 'sn_crypto_stream_xor_STATEBYTES' bytes")

  sodium_memzero(state->n, sizeof(state->n));
  sodium_memzero(state->k, sizeof(state->k));
  sodium_memzero(state->next_block, sizeof(state->next_block));
  state->remainder = 0;

  return NULL;
}

typedef struct sn_crypto_stream_chacha20_xor_state {
  unsigned char n[crypto_stream_chacha20_NONCEBYTES];
  unsigned char k[crypto_stream_chacha20_KEYBYTES];
  unsigned char next_block[64];
  int remainder;
  uint64_t block_counter;
} sn_crypto_stream_chacha20_xor_state;

napi_value sn_crypto_stream_chacha20_xor_wrap_init (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_chacha20_xor_instance_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_chacha20_xor_state *, state, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_chacha20_xor_state), "state must be 'crypto_stream_chacha20_xor_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_chacha20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_chacha20_KEYBYTES, "k")

  state->remainder = 0;
  state->block_counter = 0;
  memcpy(state->n, n_data, crypto_stream_chacha20_NONCEBYTES);
  memcpy(state->k, k_data, crypto_stream_chacha20_KEYBYTES);

  return NULL;
}

napi_value sn_crypto_stream_chacha20_xor_wrap_update (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_chacha20_xor_instance_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_chacha20_xor_state *, state, 0)
  SN_ARGV_BUFFER_CAST(unsigned char *, c, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, m, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_chacha20_xor_state), "state must be 'crypto_stream_chacha20_xor_STATEBYTES' bytes")
  SN_THROWS(c_size != m_size, "c must be 'm.byteLength' bytes")

  unsigned char *next_block = state->next_block;

  if (state->remainder) {
    uint64_t offset = 0;
    int rem = state->remainder;

    while (rem < 64 && offset < m_size) {
      c[offset] = next_block[rem]  ^ m[offset];
      offset++;
      rem++;
    }

    c += offset;
    m += offset;
    m_size -= offset;
    state->remainder = rem == 64 ? 0 : rem;

    if (!m_size) return NULL;
  }

  state->remainder = m_size & 63;
  m_size -= state->remainder;
  crypto_stream_chacha20_xor_ic(c, m, m_size, state->n, state->block_counter, state->k);
  state->block_counter += m_size / 64;

  if (state->remainder) {
    sodium_memzero(next_block + state->remainder, 64 - state->remainder);
    memcpy(next_block, m + m_size, state->remainder);

    crypto_stream_chacha20_xor_ic(next_block, next_block, 64, state->n, state->block_counter, state->k);
    memcpy(c + m_size, next_block, state->remainder);

    state->block_counter++;
  }

  return NULL;
}

napi_value sn_crypto_stream_chacha20_xor_wrap_final (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_stream_chacha20_xor_instance_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_chacha20_xor_state *, state, 0)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_chacha20_xor_state), "state must be 'crypto_stream_chacha20_xor_STATEBYTES' bytes")

  sodium_memzero(state->n, sizeof(state->n));
  sodium_memzero(state->k, sizeof(state->k));
  sodium_memzero(state->next_block, sizeof(state->next_block));
  state->remainder = 0;

  return NULL;
}

typedef struct sn_crypto_stream_chacha20_ietf_xor_state {
  unsigned char n[crypto_stream_chacha20_ietf_NONCEBYTES];
  unsigned char k[crypto_stream_chacha20_ietf_KEYBYTES];
  unsigned char next_block[64];
  int remainder;
  uint64_t block_counter;
} sn_crypto_stream_chacha20_ietf_xor_state;

napi_value sn_crypto_stream_chacha20_ietf_xor_wrap_init (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_chacha20_ietf_xor_wrap_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_chacha20_ietf_xor_state *, state, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_chacha20_ietf_xor_state), "state must be 'crypto_stream_chacha20_ietf_xor_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_chacha20_ietf_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_chacha20_ietf_KEYBYTES, "k")

  state->remainder = 0;
  state->block_counter = 0;
  memcpy(state->n, n_data, crypto_stream_chacha20_ietf_NONCEBYTES);
  memcpy(state->k, k_data, crypto_stream_chacha20_ietf_KEYBYTES);

  return NULL;
}

napi_value sn_crypto_stream_chacha20_ietf_xor_wrap_update (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_chacha20_ietf_xor_wrap_update)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_chacha20_ietf_xor_state *, state, 0)
  SN_ARGV_BUFFER_CAST(unsigned char *, c, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, m, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_chacha20_ietf_xor_state), "state must be 'crypto_stream_chacha20_ietf_xor_STATEBYTES' bytes")
  SN_THROWS(c_size != m_size, "c must be 'm.byteLength' bytes")

  unsigned char *next_block = state->next_block;

  if (state->remainder) {
    uint64_t offset = 0;
    int rem = state->remainder;

    while (rem < 64 && offset < m_size) {
      c[offset] = next_block[rem]  ^ m[offset];
      offset++;
      rem++;
    }

    c += offset;
    m += offset;
    m_size -= offset;
    state->remainder = rem == 64 ? 0 : rem;

    if (!m_size) return NULL;
  }

  state->remainder = m_size & 63;
  m_size -= state->remainder;
  crypto_stream_chacha20_ietf_xor_ic(c, m, m_size, state->n, state->block_counter, state->k);
  state->block_counter += m_size / 64;

  if (state->remainder) {
    sodium_memzero(next_block + state->remainder, 64 - state->remainder);
    memcpy(next_block, m + m_size, state->remainder);

    crypto_stream_chacha20_ietf_xor_ic(next_block, next_block, 64, state->n, state->block_counter, state->k);
    memcpy(c + m_size, next_block, state->remainder);

    state->block_counter++;
  }

  return NULL;
}

napi_value sn_crypto_stream_chacha20_ietf_xor_wrap_final (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_stream_chacha20_ietf_xor_wrap_final)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_chacha20_ietf_xor_state *, state, 0)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_chacha20_ietf_xor_state), "state must be 'crypto_stream_chacha20_ietf_xor_STATEBYTES' bytes")

  sodium_memzero(state->n, sizeof(state->n));
  sodium_memzero(state->k, sizeof(state->k));
  sodium_memzero(state->next_block, sizeof(state->next_block));
  state->remainder = 0;

  return NULL;
}

typedef struct sn_crypto_stream_xchacha20_xor_state {
  unsigned char n[crypto_stream_xchacha20_NONCEBYTES];
  unsigned char k[crypto_stream_xchacha20_KEYBYTES];
  unsigned char next_block[64];
  int remainder;
  uint64_t block_counter;
} sn_crypto_stream_xchacha20_xor_state;

napi_value sn_crypto_stream_xchacha20_xor_wrap_init (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_xchacha20_xor_wrap_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_xchacha20_xor_state *, state, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_xchacha20_xor_state), "state must be 'crypto_stream_xchacha20_xor_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_xchacha20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_xchacha20_KEYBYTES, "k")

  state->remainder = 0;
  state->block_counter = 0;
  memcpy(state->n, n_data, crypto_stream_xchacha20_NONCEBYTES);
  memcpy(state->k, k_data, crypto_stream_xchacha20_KEYBYTES);

  return NULL;
}

napi_value sn_crypto_stream_xchacha20_xor_wrap_update (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_xchacha20_xor_wrap_update)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_xchacha20_xor_state *, state, 0)
  SN_ARGV_BUFFER_CAST(unsigned char *, c, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, m, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_xchacha20_xor_state), "state must be 'crypto_stream_xchacha20_xor_STATEBYTES' bytes")
  SN_THROWS(c_size != m_size, "c must be 'm.byteLength' bytes")

  unsigned char *next_block = state->next_block;

  if (state->remainder) {
    uint64_t offset = 0;
    int rem = state->remainder;

    while (rem < 64 && offset < m_size) {
      c[offset] = next_block[rem]  ^ m[offset];
      offset++;
      rem++;
    }

    c += offset;
    m += offset;
    m_size -= offset;
    state->remainder = rem == 64 ? 0 : rem;

    if (!m_size) return NULL;
  }

  state->remainder = m_size & 63;
  m_size -= state->remainder;
  crypto_stream_xchacha20_xor_ic(c, m, m_size, state->n, state->block_counter, state->k);
  state->block_counter += m_size / 64;

  if (state->remainder) {
    sodium_memzero(next_block + state->remainder, 64 - state->remainder);
    memcpy(next_block, m + m_size, state->remainder);

    crypto_stream_xchacha20_xor_ic(next_block, next_block, 64, state->n, state->block_counter, state->k);
    memcpy(c + m_size, next_block, state->remainder);

    state->block_counter++;
  }

  return NULL;
}

napi_value sn_crypto_stream_xchacha20_xor_wrap_final (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_stream_xchacha20_xor_wrap_final)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_xchacha20_xor_state *, state, 0)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_xchacha20_xor_state), "state must be 'crypto_stream_xchacha20_xor_STATEBYTES' bytes")

  sodium_memzero(state->n, sizeof(state->n));
  sodium_memzero(state->k, sizeof(state->k));
  sodium_memzero(state->next_block, sizeof(state->next_block));
  state->remainder = 0;

  return NULL;
}

typedef struct sn_crypto_stream_salsa20_xor_state {
  unsigned char n[crypto_stream_salsa20_NONCEBYTES];
  unsigned char k[crypto_stream_salsa20_KEYBYTES];
  unsigned char next_block[64];
  int remainder;
  uint64_t block_counter;
} sn_crypto_stream_salsa20_xor_state;

napi_value sn_crypto_stream_salsa20_xor_wrap_init (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_salsa20_xor_wrap_init)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_salsa20_xor_state *, state, 0)
  SN_ARGV_TYPEDARRAY(n, 1)
  SN_ARGV_TYPEDARRAY(k, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_salsa20_xor_state), "state must be 'crypto_stream_salsa20_xor_STATEBYTES' bytes")
  SN_ASSERT_LENGTH(n_size, crypto_stream_salsa20_NONCEBYTES, "n")
  SN_ASSERT_LENGTH(k_size, crypto_stream_salsa20_KEYBYTES, "k")

  state->remainder = 0;
  state->block_counter = 0;
  memcpy(state->n, n_data, crypto_stream_salsa20_NONCEBYTES);
  memcpy(state->k, k_data, crypto_stream_salsa20_KEYBYTES);

  return NULL;
}

napi_value sn_crypto_stream_salsa20_xor_wrap_update (napi_env env, napi_callback_info info) {
  SN_ARGV(3, crypto_stream_salsa20_xor_wrap_update)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_salsa20_xor_state *, state, 0)
  SN_ARGV_BUFFER_CAST(unsigned char *, c, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, m, 2)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_salsa20_xor_state), "state must be 'crypto_stream_salsa20_xor_STATEBYTES' bytes")
  SN_THROWS(c_size != m_size, "c must be 'm.byteLength' bytes")

  unsigned char *next_block = state->next_block;

  if (state->remainder) {
    uint64_t offset = 0;
    int rem = state->remainder;

    while (rem < 64 && offset < m_size) {
      c[offset] = next_block[rem]  ^ m[offset];
      offset++;
      rem++;
    }

    c += offset;
    m += offset;
    m_size -= offset;
    state->remainder = rem == 64 ? 0 : rem;

    if (!m_size) return NULL;
  }

  state->remainder = m_size & 63;
  m_size -= state->remainder;
  crypto_stream_salsa20_xor_ic(c, m, m_size, state->n, state->block_counter, state->k);
  state->block_counter += m_size / 64;

  if (state->remainder) {
    sodium_memzero(next_block + state->remainder, 64 - state->remainder);
    memcpy(next_block, m + m_size, state->remainder);

    crypto_stream_salsa20_xor_ic(next_block, next_block, 64, state->n, state->block_counter, state->k);
    memcpy(c + m_size, next_block, state->remainder);

    state->block_counter++;
  }

  return NULL;
}

napi_value sn_crypto_stream_salsa20_xor_wrap_final (napi_env env, napi_callback_info info) {
  SN_ARGV(1, crypto_stream_salsa20_xor_wrap_final)

  SN_ARGV_BUFFER_CAST(sn_crypto_stream_salsa20_xor_state *, state, 0)

  SN_THROWS(state_size != sizeof(sn_crypto_stream_salsa20_xor_state), "state must be 'crypto_stream_salsa20_xor_STATEBYTES' bytes")

  sodium_memzero(state->n, sizeof(state->n));
  sodium_memzero(state->k, sizeof(state->k));
  sodium_memzero(state->next_block, sizeof(state->next_block));
  state->remainder = 0;

  return NULL;
}

// Experimental API

napi_value sn_extension_tweak_ed25519_base (napi_env env, napi_callback_info info) {
  SN_ARGV(3, extension_tweak_ed25519_base)

  SN_ARGV_TYPEDARRAY(n, 0)
  SN_ARGV_TYPEDARRAY(p, 1)
  SN_ARGV_TYPEDARRAY(ns, 2)

  SN_ASSERT_LENGTH(n_size, sn__extension_tweak_ed25519_SCALARBYTES, "n")
  SN_ASSERT_LENGTH(p_size, sn__extension_tweak_ed25519_BYTES, "p")

  sn__extension_tweak_ed25519_base(p_data, n_data, ns_data, ns_size);

  return NULL;
}

napi_value sn_extension_tweak_ed25519_sign_detached (napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(3, 4, extension_tweak_ed25519_sign_detached)

  SN_ARGV_TYPEDARRAY(sig, 0)
  SN_ARGV_TYPEDARRAY(m, 1)
  SN_ARGV_TYPEDARRAY(scalar, 2)
  SN_ARGV_OPTS_TYPEDARRAY(pk, 3)

  SN_ASSERT_LENGTH(sig_size, crypto_sign_BYTES, "sig")
  SN_ASSERT_LENGTH(scalar_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar")

  if (pk_data != NULL) {
    SN_ASSERT_LENGTH(pk_size, crypto_sign_PUBLICKEYBYTES, "pk")
  }

  SN_RETURN(sn__extension_tweak_ed25519_sign_detached(sig_data, NULL, m_data, m_size, scalar_data, pk_data), "failed to compute signature")
}

napi_value sn_extension_tweak_ed25519_sk_to_scalar (napi_env env, napi_callback_info info) {
  SN_ARGV(2, extension_tweak_ed25519_sk_to_scalar)

  SN_ARGV_TYPEDARRAY(n, 0)
  SN_ARGV_TYPEDARRAY(sk, 1)

  SN_ASSERT_LENGTH(n_size, sn__extension_tweak_ed25519_SCALARBYTES, "n")
  SN_ASSERT_LENGTH(sk_size, crypto_sign_SECRETKEYBYTES, "sk")

  sn__extension_tweak_ed25519_sk_to_scalar(n_data, sk_data);

  return NULL;
}

napi_value sn_extension_tweak_ed25519_scalar (napi_env env, napi_callback_info info) {
  SN_ARGV(3, extension_tweak_ed25519_scalar)

  SN_ARGV_TYPEDARRAY(scalar_out, 0)
  SN_ARGV_TYPEDARRAY(scalar, 1)
  SN_ARGV_TYPEDARRAY(ns, 2)

  SN_ASSERT_LENGTH(scalar_out_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar_out")
  SN_ASSERT_LENGTH(scalar_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar")

  sn__extension_tweak_ed25519_scalar(scalar_out_data, scalar_data, ns_data, ns_size);

  return NULL;
}

napi_value sn_extension_tweak_ed25519_pk (napi_env env, napi_callback_info info) {
  SN_ARGV(3, extension_tweak_ed25519_pk)

  SN_ARGV_TYPEDARRAY(tpk, 0)
  SN_ARGV_TYPEDARRAY(pk, 1)
  SN_ARGV_TYPEDARRAY(ns, 2)

  SN_ASSERT_LENGTH(tpk_size, crypto_sign_PUBLICKEYBYTES, "tpk")
  SN_ASSERT_LENGTH(pk_size, crypto_sign_PUBLICKEYBYTES, "pk")

  SN_RETURN(sn__extension_tweak_ed25519_pk(tpk_data, pk_data, ns_data, ns_size), "failed to tweak public key")
}

napi_value sn_extension_tweak_ed25519_keypair (napi_env env, napi_callback_info info) {
  SN_ARGV(4, extension_tweak_ed25519_keypair)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(scalar_out, 1)
  SN_ARGV_TYPEDARRAY(scalar_in, 2)
  SN_ARGV_TYPEDARRAY(ns, 3)

  SN_ASSERT_LENGTH(pk_size, sn__extension_tweak_ed25519_BYTES, "pk")
  SN_ASSERT_LENGTH(scalar_out_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar_out")
  SN_ASSERT_LENGTH(scalar_in_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar_in")

  sn__extension_tweak_ed25519_keypair(pk_data, scalar_out_data, scalar_in_data, ns_data, ns_size);

  return NULL;
}

napi_value sn_extension_tweak_ed25519_scalar_add (napi_env env, napi_callback_info info) {
  SN_ARGV(3, extension_tweak_ed25519_scalar_add)

  SN_ARGV_TYPEDARRAY(scalar_out, 0)
  SN_ARGV_TYPEDARRAY(scalar, 1)
  SN_ARGV_TYPEDARRAY(n, 2)

  SN_ASSERT_LENGTH(scalar_out_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar_out")
  SN_ASSERT_LENGTH(scalar_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar")
  SN_ASSERT_LENGTH(n_size, sn__extension_tweak_ed25519_SCALARBYTES, "n")

  sn__extension_tweak_ed25519_scalar_add(scalar_out_data, scalar_data, n_data);

  return NULL;
}

napi_value sn_extension_tweak_ed25519_pk_add (napi_env env, napi_callback_info info) {
  SN_ARGV(3, extension_tweak_ed25519_pk)

  SN_ARGV_TYPEDARRAY(tpk, 0)
  SN_ARGV_TYPEDARRAY(pk, 1)
  SN_ARGV_TYPEDARRAY(p, 2)

  SN_ASSERT_LENGTH(tpk_size, crypto_sign_PUBLICKEYBYTES, "tpk")
  SN_ASSERT_LENGTH(pk_size, crypto_sign_PUBLICKEYBYTES, "pk")
  SN_ASSERT_LENGTH(p_size, crypto_sign_PUBLICKEYBYTES, "p")

  SN_RETURN(sn__extension_tweak_ed25519_pk_add(tpk_data, pk_data, p_data), "failed to add tweak to public key")
}

napi_value sn_extension_tweak_ed25519_keypair_add (napi_env env, napi_callback_info info) {
  SN_ARGV(4, extension_tweak_ed25519_keypair_add)

  SN_ARGV_TYPEDARRAY(pk, 0)
  SN_ARGV_TYPEDARRAY(scalar_out, 1)
  SN_ARGV_TYPEDARRAY(scalar_in, 2)
  SN_ARGV_TYPEDARRAY(tweak, 3)

  SN_ASSERT_LENGTH(pk_size, sn__extension_tweak_ed25519_BYTES, "pk")
  SN_ASSERT_LENGTH(scalar_out_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar_out")
  SN_ASSERT_LENGTH(scalar_in_size, sn__extension_tweak_ed25519_SCALARBYTES, "scalar_in")
  SN_ASSERT_LENGTH(tweak_size, sn__extension_tweak_ed25519_SCALARBYTES, "tweak")

  SN_RETURN(sn__extension_tweak_ed25519_keypair_add(pk_data, scalar_out_data, scalar_in_data, tweak_data), "failed to add tweak to keypair")
}

napi_value sn_extension_pbkdf2_sha512 (napi_env env, napi_callback_info info) {
  SN_ARGV(5, extension_pbkdf2_sha512)

  SN_ARGV_BUFFER_CAST(unsigned char *, out, 0)
  SN_ARGV_BUFFER_CAST(unsigned char *, passwd, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, salt, 2)
  SN_ARGV_UINT64(iter, 3)
  SN_ARGV_UINT64(outlen, 4)

  SN_ASSERT_MIN_LENGTH(iter, sn__extension_pbkdf2_sha512_ITERATIONS_MIN, "iterations")
  SN_ASSERT_MAX_LENGTH(outlen, sn__extension_pbkdf2_sha512_BYTES_MAX, "outlen")

  SN_ASSERT_MIN_LENGTH(out_size, outlen, "out")

  SN_RETURN(sn__extension_pbkdf2_sha512(passwd, passwd_size, salt, salt_size, iter, out, outlen), "failed to add tweak to public key")
}

typedef struct sn_async_pbkdf2_sha512_request {
  napi_env env;
  unsigned char *out_data;
  size_t out_size;
  napi_ref out_ref;
  size_t outlen;
  napi_ref pwd_ref;
  const unsigned char *pwd_data;
  size_t pwd_size;
  napi_ref salt_ref;
  unsigned char *salt_data;
  size_t salt_size;
  uint64_t iter;
} sn_async_pbkdf2_sha512_request;

static void async_pbkdf2_sha512_execute (uv_work_t *uv_req) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pbkdf2_sha512_request *req = (sn_async_pbkdf2_sha512_request *) task->req;
  task->code = sn__extension_pbkdf2_sha512(req->pwd_data,
                                    req->pwd_size,
                                    req->salt_data,
                                    req->salt_size,
                                    req->iter,
                                    req->out_data,
                                    req->outlen);
}

static void async_pbkdf2_sha512_complete (uv_work_t *uv_req, int status) {
  sn_async_task_t *task = (sn_async_task_t *) uv_req;
  sn_async_pbkdf2_sha512_request *req = (sn_async_pbkdf2_sha512_request *) task->req;

  napi_handle_scope scope;
  napi_open_handle_scope(req->env, &scope);

  napi_value global;
  napi_get_global(req->env, &global);

  SN_ASYNC_COMPLETE("failed to compute kdf")

  napi_close_handle_scope(req->env, scope);

  napi_delete_reference(req->env, req->out_ref);
  napi_delete_reference(req->env, req->pwd_ref);
  napi_delete_reference(req->env, req->salt_ref);

  free(req);
  free(task);
}

napi_value sn_extension_pbkdf2_sha512_async (napi_env env, napi_callback_info info) {
  SN_ARGV_OPTS(5, 6, extension_pbkdf2_sha512_async)

  SN_ARGV_BUFFER_CAST(unsigned char *, out, 0)
  SN_ARGV_BUFFER_CAST(unsigned char *, pwd, 1)
  SN_ARGV_BUFFER_CAST(unsigned char *, salt, 2)
  SN_ARGV_UINT64(iter, 3)
  SN_ARGV_UINT64(outlen, 4)

  SN_ASSERT_MIN_LENGTH(iter, sn__extension_pbkdf2_sha512_ITERATIONS_MIN, "iterations")
  SN_ASSERT_MAX_LENGTH(outlen, sn__extension_pbkdf2_sha512_BYTES_MAX, "outlen")
  SN_ASSERT_MIN_LENGTH(out_size, outlen, "output")

  sn_async_pbkdf2_sha512_request *req = (sn_async_pbkdf2_sha512_request *) malloc(sizeof(sn_async_pbkdf2_sha512_request));

  req->env = env;
  req->out_data = out;
  req->out_size = out_size;
  req->pwd_data = pwd;
  req->pwd_size = pwd_size;
  req->salt_data = salt;
  req->salt_size = salt_size;
  req->iter = iter;
  req->outlen = outlen;

  sn_async_task_t *task = (sn_async_task_t *) malloc(sizeof(sn_async_task_t));
  SN_ASYNC_TASK(5);

  SN_STATUS_THROWS(napi_create_reference(env, out_argv, 1, &req->out_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, pwd_argv, 1, &req->pwd_ref), "")
  SN_STATUS_THROWS(napi_create_reference(env, salt_argv, 1, &req->salt_ref), "")

  SN_QUEUE_TASK(task, async_pbkdf2_sha512_execute, async_pbkdf2_sha512_complete)

  return promise;
}

static napi_value create_sodium_native (napi_env env) {
  SN_THROWS(sodium_init() == -1, "sodium_init() failed")

  napi_value exports;
  napi_create_object(env, &exports);

  // memory

  SN_EXPORT_FUNCTION(sodium_memzero, sn_sodium_memzero)
  SN_EXPORT_FUNCTION(sodium_mlock, sn_sodium_mlock)
  SN_EXPORT_FUNCTION(sodium_munlock, sn_sodium_munlock)
  SN_EXPORT_FUNCTION(sodium_malloc, sn_sodium_malloc)
  SN_EXPORT_FUNCTION(sodium_free, sn_sodium_free)
  SN_EXPORT_FUNCTION(sodium_mprotect_noaccess, sn_sodium_mprotect_noaccess)
  SN_EXPORT_FUNCTION(sodium_mprotect_readonly, sn_sodium_mprotect_readonly)
  SN_EXPORT_FUNCTION(sodium_mprotect_readwrite, sn_sodium_mprotect_readwrite)

  // randombytes

  SN_EXPORT_FUNCTION(randombytes_buf, sn_randombytes_buf)
  SN_EXPORT_FUNCTION(randombytes_buf_deterministic, sn_randombytes_buf_deterministic)
  SN_EXPORT_FUNCTION(randombytes_uniform, sn_randombytes_uniform)
  SN_EXPORT_FUNCTION(randombytes_random, sn_randombytes_random)
  SN_EXPORT_UINT32(randombytes_SEEDBYTES, randombytes_SEEDBYTES)

  // sodium helpers

  SN_EXPORT_FUNCTION(sodium_memcmp, sn_sodium_memcmp)
  SN_EXPORT_FUNCTION(sodium_increment, sn_sodium_increment)
  SN_EXPORT_FUNCTION(sodium_add, sn_sodium_add)
  SN_EXPORT_FUNCTION(sodium_sub, sn_sodium_sub)
  SN_EXPORT_FUNCTION(sodium_compare, sn_sodium_compare)
  SN_EXPORT_FUNCTION(sodium_is_zero, sn_sodium_is_zero)
  SN_EXPORT_FUNCTION(sodium_pad, sn_sodium_pad)
  SN_EXPORT_FUNCTION(sodium_unpad, sn_sodium_unpad)

  // crypto_aead

  SN_EXPORT_FUNCTION(crypto_aead_xchacha20poly1305_ietf_keygen, sn_crypto_aead_xchacha20poly1305_ietf_keygen)
  SN_EXPORT_FUNCTION(crypto_aead_xchacha20poly1305_ietf_encrypt, sn_crypto_aead_xchacha20poly1305_ietf_encrypt)
  SN_EXPORT_FUNCTION(crypto_aead_xchacha20poly1305_ietf_decrypt, sn_crypto_aead_xchacha20poly1305_ietf_decrypt)
  SN_EXPORT_FUNCTION(crypto_aead_xchacha20poly1305_ietf_encrypt_detached, sn_crypto_aead_xchacha20poly1305_ietf_encrypt_detached)
  SN_EXPORT_FUNCTION(crypto_aead_xchacha20poly1305_ietf_decrypt_detached, sn_crypto_aead_xchacha20poly1305_ietf_decrypt_detached)
  SN_EXPORT_UINT32(crypto_aead_xchacha20poly1305_ietf_ABYTES, crypto_aead_xchacha20poly1305_ietf_ABYTES)
  SN_EXPORT_UINT32(crypto_aead_xchacha20poly1305_ietf_KEYBYTES, crypto_aead_xchacha20poly1305_ietf_KEYBYTES)
  SN_EXPORT_UINT32(crypto_aead_xchacha20poly1305_ietf_NPUBBYTES, crypto_aead_xchacha20poly1305_ietf_NPUBBYTES)
  SN_EXPORT_UINT32(crypto_aead_xchacha20poly1305_ietf_NSECBYTES, crypto_aead_xchacha20poly1305_ietf_NSECBYTES)
  SN_EXPORT_UINT64(crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX, crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX)

  SN_EXPORT_FUNCTION(crypto_aead_chacha20poly1305_ietf_keygen, sn_crypto_aead_chacha20poly1305_ietf_keygen)
  SN_EXPORT_FUNCTION(crypto_aead_chacha20poly1305_ietf_encrypt, sn_crypto_aead_chacha20poly1305_ietf_encrypt)
  SN_EXPORT_FUNCTION(crypto_aead_chacha20poly1305_ietf_decrypt, sn_crypto_aead_chacha20poly1305_ietf_decrypt)
  SN_EXPORT_FUNCTION(crypto_aead_chacha20poly1305_ietf_encrypt_detached, sn_crypto_aead_chacha20poly1305_ietf_encrypt_detached)
  SN_EXPORT_FUNCTION(crypto_aead_chacha20poly1305_ietf_decrypt_detached, sn_crypto_aead_chacha20poly1305_ietf_decrypt_detached)
  SN_EXPORT_UINT32(crypto_aead_chacha20poly1305_ietf_ABYTES, crypto_aead_chacha20poly1305_ietf_ABYTES)
  SN_EXPORT_UINT32(crypto_aead_chacha20poly1305_ietf_KEYBYTES, crypto_aead_chacha20poly1305_ietf_KEYBYTES)
  SN_EXPORT_UINT32(crypto_aead_chacha20poly1305_ietf_NPUBBYTES, crypto_aead_chacha20poly1305_ietf_NPUBBYTES)
  SN_EXPORT_UINT32(crypto_aead_chacha20poly1305_ietf_NSECBYTES, crypto_aead_chacha20poly1305_ietf_NSECBYTES)
  SN_EXPORT_UINT64(crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX, crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX)

  // crypto_auth

  SN_EXPORT_FUNCTION(crypto_auth, sn_crypto_auth)
  SN_EXPORT_FUNCTION(crypto_auth_verify, sn_crypto_auth_verify)
  SN_EXPORT_UINT32(crypto_auth_BYTES, crypto_auth_BYTES)
  SN_EXPORT_UINT32(crypto_auth_KEYBYTES, crypto_auth_KEYBYTES)
  SN_EXPORT_STRING(crypto_auth_PRIMITIVE, crypto_auth_PRIMITIVE)

  // crypto_box

  SN_EXPORT_FUNCTION(crypto_box_keypair, sn_crypto_box_keypair)
  SN_EXPORT_FUNCTION(crypto_box_seed_keypair, sn_crypto_box_seed_keypair)
  SN_EXPORT_FUNCTION(crypto_box_easy, sn_crypto_box_easy)
  SN_EXPORT_FUNCTION(crypto_box_open_easy, sn_crypto_box_open_easy)
  SN_EXPORT_FUNCTION(crypto_box_detached, sn_crypto_box_detached)
  SN_EXPORT_FUNCTION(crypto_box_open_detached, sn_crypto_box_open_detached)
  SN_EXPORT_FUNCTION(crypto_box_seal, sn_crypto_box_seal)
  SN_EXPORT_FUNCTION(crypto_box_seal_open, sn_crypto_box_seal_open)
  SN_EXPORT_UINT32(crypto_box_SEEDBYTES, crypto_box_SEEDBYTES)
  SN_EXPORT_UINT32(crypto_box_PUBLICKEYBYTES, crypto_box_PUBLICKEYBYTES)
  SN_EXPORT_UINT32(crypto_box_SECRETKEYBYTES, crypto_box_SECRETKEYBYTES)
  SN_EXPORT_UINT32(crypto_box_NONCEBYTES, crypto_box_NONCEBYTES)
  SN_EXPORT_UINT32(crypto_box_MACBYTES, crypto_box_MACBYTES)
  SN_EXPORT_UINT32(crypto_box_SEALBYTES, crypto_box_SEALBYTES)
  SN_EXPORT_STRING(crypto_box_PRIMITIVE, crypto_box_PRIMITIVE)

  // crypto_core

  SN_EXPORT_FUNCTION(crypto_core_ed25519_is_valid_point, sn_crypto_core_ed25519_is_valid_point)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_from_uniform, sn_crypto_core_ed25519_from_uniform)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_add, sn_crypto_core_ed25519_add)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_sub, sn_crypto_core_ed25519_sub)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_scalar_random, sn_crypto_core_ed25519_scalar_random)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_scalar_reduce, sn_crypto_core_ed25519_scalar_reduce)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_scalar_invert, sn_crypto_core_ed25519_scalar_invert)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_scalar_negate, sn_crypto_core_ed25519_scalar_negate)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_scalar_complement, sn_crypto_core_ed25519_scalar_complement)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_scalar_add, sn_crypto_core_ed25519_scalar_add)
  SN_EXPORT_FUNCTION(crypto_core_ed25519_scalar_sub, sn_crypto_core_ed25519_scalar_sub)
  SN_EXPORT_UINT32(crypto_core_ed25519_BYTES, crypto_core_ed25519_BYTES)
  SN_EXPORT_UINT32(crypto_core_ed25519_UNIFORMBYTES, crypto_core_ed25519_UNIFORMBYTES)
  SN_EXPORT_UINT32(crypto_core_ed25519_SCALARBYTES, crypto_core_ed25519_SCALARBYTES)
  SN_EXPORT_UINT32(crypto_core_ed25519_NONREDUCEDSCALARBYTES, crypto_core_ed25519_NONREDUCEDSCALARBYTES)

  // crypto_kdf

  SN_EXPORT_FUNCTION(crypto_kdf_keygen, sn_crypto_kdf_keygen)
  SN_EXPORT_FUNCTION(crypto_kdf_derive_from_key, sn_crypto_kdf_derive_from_key)
  SN_EXPORT_UINT32(crypto_kdf_BYTES_MIN, crypto_kdf_BYTES_MIN)
  SN_EXPORT_UINT32(crypto_kdf_BYTES_MAX, crypto_kdf_BYTES_MAX)
  SN_EXPORT_UINT32(crypto_kdf_CONTEXTBYTES, crypto_kdf_CONTEXTBYTES)
  SN_EXPORT_UINT32(crypto_kdf_KEYBYTES, crypto_kdf_KEYBYTES)
  SN_EXPORT_STRING(crypto_kdf_PRIMITIVE, crypto_kdf_PRIMITIVE)

  // crypto_kx

  SN_EXPORT_FUNCTION(crypto_kx_keypair, sn_crypto_kx_keypair)
  SN_EXPORT_FUNCTION(crypto_kx_seed_keypair, sn_crypto_kx_seed_keypair)
  SN_EXPORT_FUNCTION(crypto_kx_client_session_keys, sn_crypto_kx_client_session_keys)
  SN_EXPORT_FUNCTION(crypto_kx_server_session_keys, sn_crypto_kx_server_session_keys)
  SN_EXPORT_UINT32(crypto_kx_PUBLICKEYBYTES, crypto_kx_PUBLICKEYBYTES)
  SN_EXPORT_UINT32(crypto_kx_SECRETKEYBYTES, crypto_kx_SECRETKEYBYTES)
  SN_EXPORT_UINT32(crypto_kx_SEEDBYTES, crypto_kx_SEEDBYTES)
  SN_EXPORT_UINT32(crypto_kx_SESSIONKEYBYTES, crypto_kx_SESSIONKEYBYTES)
  SN_EXPORT_STRING(crypto_kx_PRIMITIVE, crypto_kx_PRIMITIVE)

  // crypto_generichash

  SN_EXPORT_FUNCTION(crypto_generichash, sn_crypto_generichash)
  SN_EXPORT_FUNCTION(crypto_generichash_batch, sn_crypto_generichash_batch)
  SN_EXPORT_FUNCTION(crypto_generichash_keygen, sn_crypto_generichash_keygen)
  SN_EXPORT_FUNCTION(crypto_generichash_init, sn_crypto_generichash_init)
  SN_EXPORT_FUNCTION(crypto_generichash_update, sn_crypto_generichash_update)
  SN_EXPORT_FUNCTION(crypto_generichash_final, sn_crypto_generichash_final)
  SN_EXPORT_UINT32(crypto_generichash_STATEBYTES, sizeof(crypto_generichash_state))
  SN_EXPORT_STRING(crypto_generichash_PRIMITIVE, crypto_generichash_PRIMITIVE)
  SN_EXPORT_UINT32(crypto_generichash_BYTES_MIN, crypto_generichash_BYTES_MIN)
  SN_EXPORT_UINT32(crypto_generichash_BYTES_MAX, crypto_generichash_BYTES_MAX)
  SN_EXPORT_UINT32(crypto_generichash_BYTES, crypto_generichash_BYTES)
  SN_EXPORT_UINT32(crypto_generichash_KEYBYTES_MIN, crypto_generichash_KEYBYTES_MIN)
  SN_EXPORT_UINT32(crypto_generichash_KEYBYTES_MAX, crypto_generichash_KEYBYTES_MAX)
  SN_EXPORT_UINT32(crypto_generichash_KEYBYTES, crypto_generichash_KEYBYTES)

  // crypto_hash

  SN_EXPORT_FUNCTION(crypto_hash, sn_crypto_hash)
  SN_EXPORT_UINT32(crypto_hash_BYTES, crypto_hash_BYTES)
  SN_EXPORT_STRING(crypto_hash_PRIMITIVE, crypto_hash_PRIMITIVE)

  SN_EXPORT_FUNCTION(crypto_hash_sha256, sn_crypto_hash_sha256)
  SN_EXPORT_FUNCTION(crypto_hash_sha256_init, sn_crypto_hash_sha256_init)
  SN_EXPORT_FUNCTION(crypto_hash_sha256_update, sn_crypto_hash_sha256_update)
  SN_EXPORT_FUNCTION(crypto_hash_sha256_final, sn_crypto_hash_sha256_final)
  SN_EXPORT_UINT32(crypto_hash_sha256_STATEBYTES, sizeof(crypto_hash_sha256_state))
  SN_EXPORT_UINT32(crypto_hash_sha256_BYTES, crypto_hash_sha256_BYTES)

  SN_EXPORT_FUNCTION(crypto_hash_sha512, sn_crypto_hash_sha512)
  SN_EXPORT_FUNCTION(crypto_hash_sha512_init, sn_crypto_hash_sha512_init)
  SN_EXPORT_FUNCTION(crypto_hash_sha512_update, sn_crypto_hash_sha512_update)
  SN_EXPORT_FUNCTION(crypto_hash_sha512_final, sn_crypto_hash_sha512_final)
  SN_EXPORT_UINT32(crypto_hash_sha512_STATEBYTES, sizeof(crypto_hash_sha512_state))
  SN_EXPORT_UINT32(crypto_hash_sha512_BYTES, crypto_hash_sha512_BYTES)

  // crypto_onetimeauth

  SN_EXPORT_FUNCTION(crypto_onetimeauth, sn_crypto_onetimeauth)
  SN_EXPORT_FUNCTION(crypto_onetimeauth_verify, sn_crypto_onetimeauth_verify)
  SN_EXPORT_FUNCTION(crypto_onetimeauth_init, sn_crypto_onetimeauth_init)
  SN_EXPORT_FUNCTION(crypto_onetimeauth_update, sn_crypto_onetimeauth_update)
  SN_EXPORT_FUNCTION(crypto_onetimeauth_final, sn_crypto_onetimeauth_final)
  SN_EXPORT_UINT32(crypto_onetimeauth_STATEBYTES, sizeof(crypto_onetimeauth_state))
  SN_EXPORT_UINT32(crypto_onetimeauth_BYTES, crypto_onetimeauth_BYTES)
  SN_EXPORT_UINT32(crypto_onetimeauth_KEYBYTES, crypto_onetimeauth_KEYBYTES)
  SN_EXPORT_STRING(crypto_onetimeauth_PRIMITIVE, crypto_onetimeauth_PRIMITIVE)

  // crypto_pwhash

  SN_EXPORT_FUNCTION(crypto_pwhash, sn_crypto_pwhash)
  SN_EXPORT_FUNCTION(crypto_pwhash_str, sn_crypto_pwhash_str)
  SN_EXPORT_FUNCTION(crypto_pwhash_str_verify, sn_crypto_pwhash_str_verify)
  SN_EXPORT_FUNCTION(crypto_pwhash_str_needs_rehash, sn_crypto_pwhash_str_needs_rehash)
  SN_EXPORT_FUNCTION(crypto_pwhash_async, sn_crypto_pwhash_async)
  SN_EXPORT_FUNCTION(crypto_pwhash_str_async, sn_crypto_pwhash_str_async)
  SN_EXPORT_FUNCTION(crypto_pwhash_str_verify_async, sn_crypto_pwhash_str_verify_async)
  SN_EXPORT_UINT32(crypto_pwhash_ALG_ARGON2I13, crypto_pwhash_ALG_ARGON2I13)
  SN_EXPORT_UINT32(crypto_pwhash_ALG_ARGON2ID13, crypto_pwhash_ALG_ARGON2ID13)
  SN_EXPORT_UINT32(crypto_pwhash_ALG_DEFAULT, crypto_pwhash_ALG_DEFAULT)
  SN_EXPORT_UINT32(crypto_pwhash_BYTES_MIN, crypto_pwhash_BYTES_MIN)
  SN_EXPORT_UINT32(crypto_pwhash_BYTES_MAX, crypto_pwhash_BYTES_MAX)
  SN_EXPORT_UINT32(crypto_pwhash_PASSWD_MIN, crypto_pwhash_PASSWD_MIN)
  SN_EXPORT_UINT32(crypto_pwhash_PASSWD_MAX, crypto_pwhash_PASSWD_MAX)
  SN_EXPORT_UINT32(crypto_pwhash_SALTBYTES, crypto_pwhash_SALTBYTES)
  SN_EXPORT_UINT32(crypto_pwhash_STRBYTES, crypto_pwhash_STRBYTES)
  SN_EXPORT_STRING(crypto_pwhash_STRPREFIX, crypto_pwhash_STRPREFIX)
  SN_EXPORT_UINT32(crypto_pwhash_OPSLIMIT_MIN, crypto_pwhash_OPSLIMIT_MIN)
  SN_EXPORT_UINT32(crypto_pwhash_OPSLIMIT_MAX, crypto_pwhash_OPSLIMIT_MAX)
  SN_EXPORT_UINT32(crypto_pwhash_MEMLIMIT_MIN, crypto_pwhash_MEMLIMIT_MIN)
  SN_EXPORT_UINT32(crypto_pwhash_MEMLIMIT_MAX, crypto_pwhash_MEMLIMIT_MAX)
  SN_EXPORT_UINT32(crypto_pwhash_OPSLIMIT_INTERACTIVE, crypto_pwhash_OPSLIMIT_INTERACTIVE)
  SN_EXPORT_UINT32(crypto_pwhash_MEMLIMIT_INTERACTIVE, crypto_pwhash_MEMLIMIT_INTERACTIVE)
  SN_EXPORT_UINT32(crypto_pwhash_OPSLIMIT_MODERATE, crypto_pwhash_OPSLIMIT_MODERATE)
  SN_EXPORT_UINT32(crypto_pwhash_MEMLIMIT_MODERATE, crypto_pwhash_MEMLIMIT_MODERATE)
  SN_EXPORT_UINT32(crypto_pwhash_OPSLIMIT_SENSITIVE, crypto_pwhash_OPSLIMIT_SENSITIVE)
  SN_EXPORT_UINT32(crypto_pwhash_MEMLIMIT_SENSITIVE, crypto_pwhash_MEMLIMIT_SENSITIVE)
  SN_EXPORT_STRING(crypto_pwhash_PRIMITIVE, crypto_pwhash_PRIMITIVE)

  SN_EXPORT_FUNCTION(crypto_pwhash_scryptsalsa208sha256, sn_crypto_pwhash_scryptsalsa208sha256)
  SN_EXPORT_FUNCTION(crypto_pwhash_scryptsalsa208sha256_str, sn_crypto_pwhash_scryptsalsa208sha256_str)
  SN_EXPORT_FUNCTION(crypto_pwhash_scryptsalsa208sha256_str_verify, sn_crypto_pwhash_scryptsalsa208sha256_str_verify)
  SN_EXPORT_FUNCTION(crypto_pwhash_scryptsalsa208sha256_str_needs_rehash, sn_crypto_pwhash_scryptsalsa208sha256_str_needs_rehash)
  SN_EXPORT_FUNCTION(crypto_pwhash_scryptsalsa208sha256_async, sn_crypto_pwhash_scryptsalsa208sha256_async)
  SN_EXPORT_FUNCTION(crypto_pwhash_scryptsalsa208sha256_str_async, sn_crypto_pwhash_scryptsalsa208sha256_str_async)
  SN_EXPORT_FUNCTION(crypto_pwhash_scryptsalsa208sha256_str_verify_async, sn_crypto_pwhash_scryptsalsa208sha256_str_verify_async)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_BYTES_MIN, crypto_pwhash_scryptsalsa208sha256_BYTES_MIN)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_BYTES_MAX, crypto_pwhash_scryptsalsa208sha256_BYTES_MAX)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN, crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX, crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_SALTBYTES, crypto_pwhash_scryptsalsa208sha256_SALTBYTES)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_STRBYTES, crypto_pwhash_scryptsalsa208sha256_STRBYTES)
  SN_EXPORT_STRING(crypto_pwhash_scryptsalsa208sha256_STRPREFIX, crypto_pwhash_scryptsalsa208sha256_STRPREFIX)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN, crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX, crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE, crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE, crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE)
  SN_EXPORT_UINT32(crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE, crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE)

  // crypto_scalarmult

  SN_EXPORT_FUNCTION(crypto_scalarmult_base, sn_crypto_scalarmult_base)
  SN_EXPORT_FUNCTION(crypto_scalarmult, sn_crypto_scalarmult)
  SN_EXPORT_STRING(crypto_scalarmult_PRIMITIVE, crypto_scalarmult_PRIMITIVE)
  SN_EXPORT_UINT32(crypto_scalarmult_BYTES, crypto_scalarmult_BYTES)
  SN_EXPORT_UINT32(crypto_scalarmult_SCALARBYTES, crypto_scalarmult_SCALARBYTES)

  SN_EXPORT_FUNCTION(crypto_scalarmult_ed25519_base, sn_crypto_scalarmult_ed25519_base)
  SN_EXPORT_FUNCTION(crypto_scalarmult_ed25519, sn_crypto_scalarmult_ed25519)
  SN_EXPORT_FUNCTION(crypto_scalarmult_ed25519_base_noclamp, sn_crypto_scalarmult_ed25519_base_noclamp)
  SN_EXPORT_FUNCTION(crypto_scalarmult_ed25519_noclamp, sn_crypto_scalarmult_ed25519_noclamp)
  SN_EXPORT_UINT32(crypto_scalarmult_ed25519_BYTES, crypto_scalarmult_ed25519_BYTES)
  SN_EXPORT_UINT32(crypto_scalarmult_ed25519_SCALARBYTES, crypto_scalarmult_ed25519_SCALARBYTES)

  // crypto_secretbox

  SN_EXPORT_FUNCTION(crypto_secretbox_easy, sn_crypto_secretbox_easy)
  SN_EXPORT_FUNCTION(crypto_secretbox_open_easy, sn_crypto_secretbox_open_easy)
  SN_EXPORT_FUNCTION(crypto_secretbox_detached, sn_crypto_secretbox_detached)
  SN_EXPORT_FUNCTION(crypto_secretbox_open_detached, sn_crypto_secretbox_open_detached)
  SN_EXPORT_UINT32(crypto_secretbox_KEYBYTES, crypto_secretbox_KEYBYTES)
  SN_EXPORT_UINT32(crypto_secretbox_NONCEBYTES, crypto_secretbox_NONCEBYTES)
  SN_EXPORT_UINT32(crypto_secretbox_MACBYTES, crypto_secretbox_MACBYTES)
  SN_EXPORT_STRING(crypto_secretbox_PRIMITIVE, crypto_secretbox_PRIMITIVE)

  // crypto_secretstream

  SN_EXPORT_FUNCTION(crypto_secretstream_xchacha20poly1305_keygen, sn_crypto_secretstream_xchacha20poly1305_keygen)
  SN_EXPORT_FUNCTION(crypto_secretstream_xchacha20poly1305_init_push, sn_crypto_secretstream_xchacha20poly1305_init_push)
  SN_EXPORT_FUNCTION(crypto_secretstream_xchacha20poly1305_init_pull, sn_crypto_secretstream_xchacha20poly1305_init_pull)
  SN_EXPORT_FUNCTION(crypto_secretstream_xchacha20poly1305_push, sn_crypto_secretstream_xchacha20poly1305_push)
  SN_EXPORT_FUNCTION(crypto_secretstream_xchacha20poly1305_pull, sn_crypto_secretstream_xchacha20poly1305_pull)
  SN_EXPORT_FUNCTION(crypto_secretstream_xchacha20poly1305_rekey, sn_crypto_secretstream_xchacha20poly1305_rekey)
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_STATEBYTES, sizeof(crypto_secretstream_xchacha20poly1305_state))
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_ABYTES, crypto_secretstream_xchacha20poly1305_ABYTES)
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_HEADERBYTES, crypto_secretstream_xchacha20poly1305_HEADERBYTES)
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_KEYBYTES, crypto_secretstream_xchacha20poly1305_KEYBYTES)
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_TAGBYTES, 1)
  SN_EXPORT_UINT64(crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX, crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX)
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_TAG_MESSAGE, crypto_secretstream_xchacha20poly1305_TAG_MESSAGE)
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_TAG_PUSH, crypto_secretstream_xchacha20poly1305_TAG_PUSH)
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_TAG_REKEY, crypto_secretstream_xchacha20poly1305_TAG_REKEY)
  SN_EXPORT_UINT32(crypto_secretstream_xchacha20poly1305_TAG_FINAL, crypto_secretstream_xchacha20poly1305_TAG_FINAL)

  // crypto_shorthash

  SN_EXPORT_FUNCTION(crypto_shorthash, sn_crypto_shorthash)
  SN_EXPORT_UINT32(crypto_shorthash_BYTES, crypto_shorthash_BYTES)
  SN_EXPORT_UINT32(crypto_shorthash_KEYBYTES, crypto_shorthash_KEYBYTES)
  SN_EXPORT_STRING(crypto_shorthash_PRIMITIVE, crypto_shorthash_PRIMITIVE)

  // crypto_sign

  SN_EXPORT_FUNCTION(crypto_sign_keypair, sn_crypto_sign_keypair)
  SN_EXPORT_FUNCTION(crypto_sign_seed_keypair, sn_crypto_sign_seed_keypair)
  SN_EXPORT_FUNCTION(crypto_sign, sn_crypto_sign)
  SN_EXPORT_FUNCTION(crypto_sign_open, sn_crypto_sign_open)
  SN_EXPORT_FUNCTION(crypto_sign_detached, sn_crypto_sign_detached)
  SN_EXPORT_FUNCTION(crypto_sign_verify_detached, sn_crypto_sign_verify_detached)
  SN_EXPORT_FUNCTION(crypto_sign_ed25519_sk_to_pk, sn_crypto_sign_ed25519_sk_to_pk)
  SN_EXPORT_FUNCTION(crypto_sign_ed25519_pk_to_curve25519, sn_crypto_sign_ed25519_pk_to_curve25519)
  SN_EXPORT_FUNCTION(crypto_sign_ed25519_sk_to_curve25519, sn_crypto_sign_ed25519_sk_to_curve25519)
  SN_EXPORT_UINT32(crypto_sign_SEEDBYTES, crypto_sign_SEEDBYTES)
  SN_EXPORT_UINT32(crypto_sign_PUBLICKEYBYTES, crypto_sign_PUBLICKEYBYTES)
  SN_EXPORT_UINT32(crypto_sign_SECRETKEYBYTES, crypto_sign_SECRETKEYBYTES)
  SN_EXPORT_UINT32(crypto_sign_BYTES, crypto_sign_BYTES)

  // crypto_stream

  SN_EXPORT_FUNCTION(crypto_stream, sn_crypto_stream)
  SN_EXPORT_UINT32(crypto_stream_KEYBYTES, crypto_stream_KEYBYTES)
  SN_EXPORT_UINT32(crypto_stream_NONCEBYTES, crypto_stream_NONCEBYTES)
  SN_EXPORT_STRING(crypto_stream_PRIMITIVE, crypto_stream_PRIMITIVE)

  SN_EXPORT_FUNCTION(crypto_stream_xor, sn_crypto_stream_xor)
  SN_EXPORT_FUNCTION(crypto_stream_xor_init, sn_crypto_stream_xor_wrap_init)
  SN_EXPORT_FUNCTION(crypto_stream_xor_update, sn_crypto_stream_xor_wrap_update)
  SN_EXPORT_FUNCTION(crypto_stream_xor_final, sn_crypto_stream_xor_wrap_final)
  SN_EXPORT_UINT32(crypto_stream_xor_STATEBYTES, sizeof(sn_crypto_stream_xor_state))

  SN_EXPORT_FUNCTION(crypto_stream_chacha20, sn_crypto_stream_chacha20)
  SN_EXPORT_UINT32(crypto_stream_chacha20_KEYBYTES, crypto_stream_chacha20_KEYBYTES)
  SN_EXPORT_UINT32(crypto_stream_chacha20_NONCEBYTES, crypto_stream_chacha20_NONCEBYTES)
  SN_EXPORT_UINT64(crypto_stream_chacha20_MESSAGEBYTES_MAX, crypto_stream_chacha20_MESSAGEBYTES_MAX)

  SN_EXPORT_FUNCTION(crypto_stream_chacha20_xor, sn_crypto_stream_chacha20_xor)
  SN_EXPORT_FUNCTION(crypto_stream_chacha20_xor_ic, sn_crypto_stream_chacha20_xor_ic)
  SN_EXPORT_FUNCTION(crypto_stream_chacha20_xor_init, sn_crypto_stream_chacha20_xor_wrap_init)
  SN_EXPORT_FUNCTION(crypto_stream_chacha20_xor_update, sn_crypto_stream_chacha20_xor_wrap_update)
  SN_EXPORT_FUNCTION(crypto_stream_chacha20_xor_final, sn_crypto_stream_chacha20_xor_wrap_final)
  SN_EXPORT_UINT32(crypto_stream_chacha20_xor_STATEBYTES, sizeof(sn_crypto_stream_chacha20_xor_state))

  SN_EXPORT_FUNCTION(crypto_stream_chacha20_ietf, sn_crypto_stream_chacha20_ietf)
  SN_EXPORT_UINT32(crypto_stream_chacha20_ietf_KEYBYTES, crypto_stream_chacha20_ietf_KEYBYTES)
  SN_EXPORT_UINT32(crypto_stream_chacha20_ietf_NONCEBYTES, crypto_stream_chacha20_ietf_NONCEBYTES)
  SN_EXPORT_UINT64(crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX, crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX)
  SN_EXPORT_UINT32(crypto_stream_chacha20_ietf_xor_STATEBYTES, sizeof(sn_crypto_stream_chacha20_ietf_xor_state))

  SN_EXPORT_FUNCTION(crypto_stream_chacha20_ietf_xor, sn_crypto_stream_chacha20_ietf_xor)
  SN_EXPORT_FUNCTION(crypto_stream_chacha20_ietf_xor_ic, sn_crypto_stream_chacha20_ietf_xor_ic)
  SN_EXPORT_FUNCTION(crypto_stream_chacha20_ietf_xor_init, sn_crypto_stream_chacha20_ietf_xor_wrap_init)
  SN_EXPORT_FUNCTION(crypto_stream_chacha20_ietf_xor_update, sn_crypto_stream_chacha20_ietf_xor_wrap_update)
  SN_EXPORT_FUNCTION(crypto_stream_chacha20_ietf_xor_final, sn_crypto_stream_chacha20_ietf_xor_wrap_final)

  SN_EXPORT_FUNCTION(crypto_stream_xchacha20, sn_crypto_stream_xchacha20)
  SN_EXPORT_UINT32(crypto_stream_xchacha20_KEYBYTES, crypto_stream_xchacha20_KEYBYTES)
  SN_EXPORT_UINT32(crypto_stream_xchacha20_NONCEBYTES, crypto_stream_xchacha20_NONCEBYTES)
  SN_EXPORT_UINT64(crypto_stream_xchacha20_MESSAGEBYTES_MAX, crypto_stream_xchacha20_MESSAGEBYTES_MAX)

  SN_EXPORT_FUNCTION(crypto_stream_xchacha20_xor, sn_crypto_stream_xchacha20_xor)
  SN_EXPORT_FUNCTION(crypto_stream_xchacha20_xor_ic, sn_crypto_stream_xchacha20_xor_ic)
  SN_EXPORT_FUNCTION(crypto_stream_xchacha20_xor_init, sn_crypto_stream_xchacha20_xor_wrap_init)
  SN_EXPORT_FUNCTION(crypto_stream_xchacha20_xor_update, sn_crypto_stream_xchacha20_xor_wrap_update)
  SN_EXPORT_FUNCTION(crypto_stream_xchacha20_xor_final, sn_crypto_stream_xchacha20_xor_wrap_final)
  SN_EXPORT_FUNCTION(crypto_stream_xchacha20, sn_crypto_stream_xchacha20)
  SN_EXPORT_UINT32(crypto_stream_xchacha20_xor_STATEBYTES, sizeof(sn_crypto_stream_xchacha20_xor_state))

  SN_EXPORT_FUNCTION(crypto_stream_salsa20, sn_crypto_stream_salsa20)
  SN_EXPORT_UINT32(crypto_stream_salsa20_KEYBYTES, crypto_stream_salsa20_KEYBYTES)
  SN_EXPORT_UINT32(crypto_stream_salsa20_NONCEBYTES, crypto_stream_salsa20_NONCEBYTES)
  SN_EXPORT_UINT64(crypto_stream_salsa20_MESSAGEBYTES_MAX, crypto_stream_salsa20_MESSAGEBYTES_MAX)

  SN_EXPORT_FUNCTION(crypto_stream_salsa20_xor, sn_crypto_stream_salsa20_xor)
  SN_EXPORT_FUNCTION(crypto_stream_salsa20_xor_ic, sn_crypto_stream_salsa20_xor_ic)
  SN_EXPORT_FUNCTION(crypto_stream_salsa20_xor_init, sn_crypto_stream_salsa20_xor_wrap_init)
  SN_EXPORT_FUNCTION(crypto_stream_salsa20_xor_update, sn_crypto_stream_salsa20_xor_wrap_update)
  SN_EXPORT_FUNCTION(crypto_stream_salsa20_xor_final, sn_crypto_stream_salsa20_xor_wrap_final)
  SN_EXPORT_UINT32(crypto_stream_salsa20_xor_STATEBYTES, sizeof(sn_crypto_stream_salsa20_xor_state))

  // extensions

  // tweak

  SN_EXPORT_FUNCTION(extension_tweak_ed25519_base, sn_extension_tweak_ed25519_base)
  SN_EXPORT_FUNCTION(extension_tweak_ed25519_sign_detached, sn_extension_tweak_ed25519_sign_detached)
  SN_EXPORT_FUNCTION(extension_tweak_ed25519_sk_to_scalar, sn_extension_tweak_ed25519_sk_to_scalar)
  SN_EXPORT_FUNCTION(extension_tweak_ed25519_scalar, sn_extension_tweak_ed25519_scalar)
  SN_EXPORT_FUNCTION(extension_tweak_ed25519_pk, sn_extension_tweak_ed25519_pk)
  SN_EXPORT_FUNCTION(extension_tweak_ed25519_keypair, sn_extension_tweak_ed25519_keypair)
  SN_EXPORT_FUNCTION(extension_tweak_ed25519_scalar_add, sn_extension_tweak_ed25519_scalar_add)
  SN_EXPORT_FUNCTION(extension_tweak_ed25519_pk_add, sn_extension_tweak_ed25519_pk_add)
  SN_EXPORT_FUNCTION(extension_tweak_ed25519_keypair_add, sn_extension_tweak_ed25519_keypair_add)
  SN_EXPORT_UINT32(extension_tweak_ed25519_BYTES, sn__extension_tweak_ed25519_BYTES)
  SN_EXPORT_UINT32(extension_tweak_ed25519_SCALARBYTES, sn__extension_tweak_ed25519_SCALARBYTES)

  // pbkdf2

  SN_EXPORT_FUNCTION(extension_pbkdf2_sha512, sn_extension_pbkdf2_sha512)
  SN_EXPORT_FUNCTION(extension_pbkdf2_sha512_async, sn_extension_pbkdf2_sha512_async)
  SN_EXPORT_UINT32(extension_pbkdf2_sha512_SALTBYTES, sn__extension_pbkdf2_sha512_SALTBYTES)
  SN_EXPORT_UINT32(extension_pbkdf2_sha512_HASHBYTES, sn__extension_pbkdf2_sha512_HASHBYTES)
  SN_EXPORT_UINT32(extension_pbkdf2_sha512_ITERATIONS_MIN, sn__extension_pbkdf2_sha512_ITERATIONS_MIN)
  SN_EXPORT_UINT64(extension_pbkdf2_sha512_BYTES_MAX, sn__extension_pbkdf2_sha512_BYTES_MAX)

  return exports;
}

static napi_value Init(napi_env env, napi_value exports) {
  return create_sodium_native(env);
}

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)
