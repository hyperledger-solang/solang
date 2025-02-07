"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
Object.defineProperty(exports, "Int128", {
  enumerable: true,
  get: function get() {
    return _int.Int128;
  }
});
Object.defineProperty(exports, "Int256", {
  enumerable: true,
  get: function get() {
    return _int2.Int256;
  }
});
Object.defineProperty(exports, "ScInt", {
  enumerable: true,
  get: function get() {
    return _sc_int.ScInt;
  }
});
Object.defineProperty(exports, "Uint128", {
  enumerable: true,
  get: function get() {
    return _uint.Uint128;
  }
});
Object.defineProperty(exports, "Uint256", {
  enumerable: true,
  get: function get() {
    return _uint2.Uint256;
  }
});
Object.defineProperty(exports, "XdrLargeInt", {
  enumerable: true,
  get: function get() {
    return _xdr_large_int.XdrLargeInt;
  }
});
exports.scValToBigInt = scValToBigInt;
var _xdr_large_int = require("./xdr_large_int");
var _uint = require("./uint128");
var _uint2 = require("./uint256");
var _int = require("./int128");
var _int2 = require("./int256");
var _sc_int = require("./sc_int");
/**
 * Transforms an opaque {@link xdr.ScVal} into a native bigint, if possible.
 *
 * If you then want to use this in the abstractions provided by this module,
 * you can pass it to the constructor of {@link XdrLargeInt}.
 *
 * @example
 * let scv = contract.call("add", x, y); // assume it returns an xdr.ScVal
 * let bigi = scValToBigInt(scv);
 *
 * new ScInt(bigi);               // if you don't care about types, and
 * new XdrLargeInt('i128', bigi); // if you do
 *
 * @param {xdr.ScVal} scv - the raw XDR value to parse into an integer
 * @returns {bigint} the native value of this input value
 *
 * @throws {TypeError} if the `scv` input value doesn't represent an integer
 */
function scValToBigInt(scv) {
  var scIntType = _xdr_large_int.XdrLargeInt.getType(scv["switch"]().name);
  switch (scv["switch"]().name) {
    case 'scvU32':
    case 'scvI32':
      return BigInt(scv.value());
    case 'scvU64':
    case 'scvI64':
      return new _xdr_large_int.XdrLargeInt(scIntType, scv.value()).toBigInt();
    case 'scvU128':
    case 'scvI128':
      return new _xdr_large_int.XdrLargeInt(scIntType, [scv.value().lo(), scv.value().hi()]).toBigInt();
    case 'scvU256':
    case 'scvI256':
      return new _xdr_large_int.XdrLargeInt(scIntType, [scv.value().loLo(), scv.value().loHi(), scv.value().hiLo(), scv.value().hiHi()]).toBigInt();
    default:
      throw TypeError("expected integer type, got ".concat(scv["switch"]()));
  }
}