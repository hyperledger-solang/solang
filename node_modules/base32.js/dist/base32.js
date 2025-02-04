this["base32"] =
/******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/
/******/ 		// Check if module is in cache
/******/ 		if(installedModules[moduleId])
/******/ 			return installedModules[moduleId].exports;
/******/
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = installedModules[moduleId] = {
/******/ 			exports: {},
/******/ 			id: moduleId,
/******/ 			loaded: false
/******/ 		};
/******/
/******/ 		// Execute the module function
/******/ 		modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/
/******/ 		// Flag the module as loaded
/******/ 		module.loaded = true;
/******/
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/
/******/
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = modules;
/******/
/******/ 	// expose the module cache
/******/ 	__webpack_require__.c = installedModules;
/******/
/******/ 	// __webpack_public_path__
/******/ 	__webpack_require__.p = "";
/******/
/******/ 	// Load entry module and return exports
/******/ 	return __webpack_require__(0);
/******/ })
/************************************************************************/
/******/ ([
/* 0 */
/***/ function(module, exports, __webpack_require__) {

"use strict";

/**
 * Generate a character map.
 * @param {string} alphabet e.g. "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
 * @param {object} mappings map overrides from key to value
 * @method
 */

var charmap = function (alphabet, mappings) {
  mappings || (mappings = {});
  alphabet.split("").forEach(function (c, i) {
    if (!(c in mappings)) mappings[c] = i;
  });
  return mappings;
}

/**
 * The RFC 4648 base 32 alphabet and character map.
 * @see {@link https://tools.ietf.org/html/rfc4648}
 */

var rfc4648 = {
  alphabet: "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
  charmap: {
    0: 14,
    1: 8
  }
};

rfc4648.charmap = charmap(rfc4648.alphabet, rfc4648.charmap);

/**
 * The Crockford base 32 alphabet and character map.
 * @see {@link http://www.crockford.com/wrmg/base32.html}
 */

var crockford = {
  alphabet: "0123456789ABCDEFGHJKMNPQRSTVWXYZ",
  charmap: {
    O: 0,
    I: 1,
    L: 1
  }
};

crockford.charmap = charmap(crockford.alphabet, crockford.charmap);

/**
 * base32hex
 * @see {@link https://en.wikipedia.org/wiki/Base32#base32hex}
 */

var base32hex = {
  alphabet: "0123456789ABCDEFGHIJKLMNOPQRSTUV",
  charmap: {}
};

base32hex.charmap = charmap(base32hex.alphabet, base32hex.charmap);

/**
 * Create a new `Decoder` with the given options.
 *
 * @param {object} [options]
 *   @param {string} [type] Supported Base-32 variants are "rfc4648" and
 *     "crockford".
 *   @param {object} [charmap] Override the character map used in decoding.
 * @constructor
 */

function Decoder (options) {
  this.buf = [];
  this.shift = 8;
  this.carry = 0;

  if (options) {

    switch (options.type) {
      case "rfc4648":
        this.charmap = exports.rfc4648.charmap;
        break;
      case "crockford":
        this.charmap = exports.crockford.charmap;
        break;
      case "base32hex":
        this.charmap = exports.base32hex.charmap;
        break;
      default:
        throw new Error("invalid type");
    }

    if (options.charmap) this.charmap = options.charmap;
  }
}

/**
 * The default character map coresponds to RFC4648.
 */

Decoder.prototype.charmap = rfc4648.charmap;

/**
 * Decode a string, continuing from the previous state.
 *
 * @param {string} str
 * @return {Decoder} this
 */

Decoder.prototype.write = function (str) {
  var charmap = this.charmap;
  var buf = this.buf;
  var shift = this.shift;
  var carry = this.carry;

  // decode string
  str.toUpperCase().split("").forEach(function (char) {

    // ignore padding
    if (char == "=") return;

    // lookup symbol
    var symbol = charmap[char] & 0xff;

    // 1: 00000 000
    // 2:          00 00000 0
    // 3:                    0000 0000
    // 4:                             0 00000 00
    // 5:                                       000 00000
    // 6:                                                00000 000
    // 7:                                                         00 00000 0

    shift -= 5;
    if (shift > 0) {
      carry |= symbol << shift;
    } else if (shift < 0) {
      buf.push(carry | (symbol >> -shift));
      shift += 8;
      carry = (symbol << shift) & 0xff;
    } else {
      buf.push(carry | symbol);
      shift = 8;
      carry = 0;
    }
  });

  // save state
  this.shift = shift;
  this.carry = carry;

  // for chaining
  return this;
};

/**
 * Finish decoding.
 *
 * @param {string} [str] The final string to decode.
 * @return {Array} Decoded byte array.
 */

Decoder.prototype.finalize = function (str) {
  if (str) {
    this.write(str);
  }
  if (this.shift !== 8 && this.carry !== 0) {
    this.buf.push(this.carry);
    this.shift = 8;
    this.carry = 0;
  }
  return this.buf;
};

/**
 * Create a new `Encoder` with the given options.
 *
 * @param {object} [options]
 *   @param {string} [type] Supported Base-32 variants are "rfc4648" and
 *     "crockford".
 *   @param {object} [alphabet] Override the alphabet used in encoding.
 * @constructor
 */

function Encoder (options) {
  this.buf = "";
  this.shift = 3;
  this.carry = 0;

  if (options) {

    switch (options.type) {
      case "rfc4648":
        this.alphabet = exports.rfc4648.alphabet;
        break;
      case "crockford":
        this.alphabet = exports.crockford.alphabet;
        break;
      case "base32hex":
        this.alphabet = exports.base32hex.alphabet;
        break;
      default:
        throw new Error("invalid type");
    }

    if (options.alphabet) this.alphabet = options.alphabet;
    else if (options.lc) this.alphabet = this.alphabet.toLowerCase();
  }
}

/**
 * The default alphabet coresponds to RFC4648.
 */

Encoder.prototype.alphabet = rfc4648.alphabet;

/**
 * Encode a byte array, continuing from the previous state.
 *
 * @param {byte[]} buf The byte array to encode.
 * @return {Encoder} this
 */

Encoder.prototype.write = function (buf) {
  var shift = this.shift;
  var carry = this.carry;
  var symbol;
  var byte;
  var i;

  // encode each byte in buf
  for (i = 0; i < buf.length; i++) {
    byte = buf[i];

    // 1: 00000 000
    // 2:          00 00000 0
    // 3:                    0000 0000
    // 4:                             0 00000 00
    // 5:                                       000 00000
    // 6:                                                00000 000
    // 7:                                                         00 00000 0

    symbol = carry | (byte >> shift);
    this.buf += this.alphabet[symbol & 0x1f];

    if (shift > 5) {
      shift -= 5;
      symbol = byte >> shift;
      this.buf += this.alphabet[symbol & 0x1f];
    }

    shift = 5 - shift;
    carry = byte << shift;
    shift = 8 - shift;
  }

  // save state
  this.shift = shift;
  this.carry = carry;

  // for chaining
  return this;
};

/**
 * Finish encoding.
 *
 * @param {byte[]} [buf] The final byte array to encode.
 * @return {string} The encoded byte array.
 */

Encoder.prototype.finalize = function (buf) {
  if (buf) {
    this.write(buf);
  }
  if (this.shift !== 3) {
    this.buf += this.alphabet[this.carry & 0x1f];
    this.shift = 3;
    this.carry = 0;
  }
  return this.buf;
};

/**
 * Convenience encoder.
 *
 * @param {byte[]} buf The byte array to encode.
 * @param {object} [options] Options to pass to the encoder.
 * @return {string} The encoded string.
 */

exports.encode = function (buf, options) {
  return new Encoder(options).finalize(buf);
};

/**
 * Convenience decoder.
 *
 * @param {string} str The string to decode.
 * @param {object} [options] Options to pass to the decoder.
 * @return {byte[]} The decoded byte array.
 */

exports.decode = function (str, options) {
  return new Decoder(options).finalize(str);
};

// Exports.
exports.Decoder = Decoder;
exports.Encoder = Encoder;
exports.charmap = charmap;
exports.crockford = crockford;
exports.rfc4648 = rfc4648;
exports.base32hex = base32hex;


/***/ }
/******/ ])
//# sourceMappingURL=base32.js.map