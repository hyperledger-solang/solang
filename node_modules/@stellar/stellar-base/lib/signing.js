"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.FastSigning = void 0;
exports.generate = generate;
exports.sign = sign;
exports.verify = verify;
//  This module provides the signing functionality used by the stellar network
//  The code below may look a little strange... this is because we try to provide
//  the most efficient signing method possible.  First, we try to load the
//  native `sodium-native` package for node.js environments, and if that fails we
//  fallback to `tweetnacl`

var actualMethods = {};

/**
 * Use this flag to check if fast signing (provided by `sodium-native` package) is available.
 * If your app is signing a large number of transaction or verifying a large number
 * of signatures make sure `sodium-native` package is installed.
 */
var FastSigning = exports.FastSigning = checkFastSigning();
function sign(data, secretKey) {
  return actualMethods.sign(data, secretKey);
}
function verify(data, signature, publicKey) {
  return actualMethods.verify(data, signature, publicKey);
}
function generate(secretKey) {
  return actualMethods.generate(secretKey);
}
function checkFastSigning() {
  return typeof window === 'undefined' ? checkFastSigningNode() : checkFastSigningBrowser();
}
function checkFastSigningNode() {
  // NOTE: we use commonjs style require here because es6 imports
  // can only occur at the top level.  thanks, obama.
  var sodium;
  try {
    // eslint-disable-next-line
    sodium = require('sodium-native');
  } catch (err) {
    return checkFastSigningBrowser();
  }
  if (!Object.keys(sodium).length) {
    return checkFastSigningBrowser();
  }
  actualMethods.generate = function (secretKey) {
    var pk = Buffer.alloc(sodium.crypto_sign_PUBLICKEYBYTES);
    var sk = Buffer.alloc(sodium.crypto_sign_SECRETKEYBYTES);
    sodium.crypto_sign_seed_keypair(pk, sk, secretKey);
    return pk;
  };
  actualMethods.sign = function (data, secretKey) {
    data = Buffer.from(data);
    var signature = Buffer.alloc(sodium.crypto_sign_BYTES);
    sodium.crypto_sign_detached(signature, data, secretKey);
    return signature;
  };
  actualMethods.verify = function (data, signature, publicKey) {
    data = Buffer.from(data);
    try {
      return sodium.crypto_sign_verify_detached(signature, data, publicKey);
    } catch (e) {
      return false;
    }
  };
  return true;
}
function checkFastSigningBrowser() {
  // fallback to `tweetnacl` if we're in the browser or
  // if there was a failure installing `sodium-native`
  // eslint-disable-next-line
  var nacl = require('tweetnacl');
  actualMethods.generate = function (secretKey) {
    var secretKeyUint8 = new Uint8Array(secretKey);
    var naclKeys = nacl.sign.keyPair.fromSeed(secretKeyUint8);
    return Buffer.from(naclKeys.publicKey);
  };
  actualMethods.sign = function (data, secretKey) {
    data = Buffer.from(data);
    data = new Uint8Array(data.toJSON().data);
    secretKey = new Uint8Array(secretKey.toJSON().data);
    var signature = nacl.sign.detached(data, secretKey);
    return Buffer.from(signature);
  };
  actualMethods.verify = function (data, signature, publicKey) {
    data = Buffer.from(data);
    data = new Uint8Array(data.toJSON().data);
    signature = new Uint8Array(signature.toJSON().data);
    publicKey = new Uint8Array(publicKey.toJSON().data);
    return nacl.sign.detached.verify(data, signature, publicKey);
  };
  return false;
}