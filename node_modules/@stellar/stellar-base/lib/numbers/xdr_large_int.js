"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.XdrLargeInt = void 0;
var _jsXdr = require("@stellar/js-xdr");
var _uint = require("./uint128");
var _uint2 = require("./uint256");
var _int = require("./int128");
var _int2 = require("./int256");
var _xdr = _interopRequireDefault(require("../xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); } /* eslint no-bitwise: ["error", {"allow": [">>"]}] */
/**
 * A wrapper class to represent large XDR-encodable integers.
 *
 * This operates at a lower level than {@link ScInt} by forcing you to specify
 * the type / width / size in bits of the integer you're targeting, regardless
 * of the input value(s) you provide.
 *
 * @param {string}  type - force a specific data type. the type choices are:
 *    'i64', 'u64', 'i128', 'u128', 'i256', and 'u256' (default: the smallest
 *    one that fits the `value`) (see {@link XdrLargeInt.isType})
 * @param {number|bigint|string|Array<number|bigint|string>} values   a list of
 *    integer-like values interpreted in big-endian order
 */
var XdrLargeInt = exports.XdrLargeInt = /*#__PURE__*/function () {
  function XdrLargeInt(type, values) {
    _classCallCheck(this, XdrLargeInt);
    /** @type {xdr.LargeInt} */
    _defineProperty(this, "int", void 0);
    // child class of a jsXdr.LargeInt
    /** @type {string} */
    _defineProperty(this, "type", void 0);
    if (!(values instanceof Array)) {
      values = [values];
    }

    // normalize values to one type
    values = values.map(function (i) {
      // micro-optimization to no-op on the likeliest input value:
      if (typeof i === 'bigint') {
        return i;
      }
      if (i instanceof XdrLargeInt) {
        return i.toBigInt();
      }
      return BigInt(i);
    });
    switch (type) {
      case 'i64':
        this["int"] = new _jsXdr.Hyper(values);
        break;
      case 'i128':
        this["int"] = new _int.Int128(values);
        break;
      case 'i256':
        this["int"] = new _int2.Int256(values);
        break;
      case 'u64':
        this["int"] = new _jsXdr.UnsignedHyper(values);
        break;
      case 'u128':
        this["int"] = new _uint.Uint128(values);
        break;
      case 'u256':
        this["int"] = new _uint2.Uint256(values);
        break;
      default:
        throw TypeError("invalid type: ".concat(type));
    }
    this.type = type;
  }

  /**
   * @returns {number}
   * @throws {RangeError} if the value can't fit into a Number
   */
  return _createClass(XdrLargeInt, [{
    key: "toNumber",
    value: function toNumber() {
      var bi = this["int"].toBigInt();
      if (bi > Number.MAX_SAFE_INTEGER || bi < Number.MIN_SAFE_INTEGER) {
        throw RangeError("value ".concat(bi, " not in range for Number ") + "[".concat(Number.MAX_SAFE_INTEGER, ", ").concat(Number.MIN_SAFE_INTEGER, "]"));
      }
      return Number(bi);
    }

    /** @returns {bigint} */
  }, {
    key: "toBigInt",
    value: function toBigInt() {
      return this["int"].toBigInt();
    }

    /** @returns {xdr.ScVal} the integer encoded with `ScValType = I64` */
  }, {
    key: "toI64",
    value: function toI64() {
      this._sizeCheck(64);
      var v = this.toBigInt();
      if (BigInt.asIntN(64, v) !== v) {
        throw RangeError("value too large for i64: ".concat(v));
      }
      return _xdr["default"].ScVal.scvI64(new _xdr["default"].Int64(v));
    }

    /** @returns {xdr.ScVal} the integer encoded with `ScValType = U64` */
  }, {
    key: "toU64",
    value: function toU64() {
      this._sizeCheck(64);
      return _xdr["default"].ScVal.scvU64(new _xdr["default"].Uint64(BigInt.asUintN(64, this.toBigInt())) // reiterpret as unsigned
      );
    }

    /**
     * @returns {xdr.ScVal} the integer encoded with `ScValType = I128`
     * @throws {RangeError} if the value cannot fit in 128 bits
     */
  }, {
    key: "toI128",
    value: function toI128() {
      this._sizeCheck(128);
      var v = this["int"].toBigInt();
      var hi64 = BigInt.asIntN(64, v >> 64n); // encode top 64 w/ sign bit
      var lo64 = BigInt.asUintN(64, v); // grab btm 64, encode sign

      return _xdr["default"].ScVal.scvI128(new _xdr["default"].Int128Parts({
        hi: new _xdr["default"].Int64(hi64),
        lo: new _xdr["default"].Uint64(lo64)
      }));
    }

    /**
     * @returns {xdr.ScVal} the integer encoded with `ScValType = U128`
     * @throws {RangeError} if the value cannot fit in 128 bits
     */
  }, {
    key: "toU128",
    value: function toU128() {
      this._sizeCheck(128);
      var v = this["int"].toBigInt();
      return _xdr["default"].ScVal.scvU128(new _xdr["default"].UInt128Parts({
        hi: new _xdr["default"].Uint64(BigInt.asUintN(64, v >> 64n)),
        lo: new _xdr["default"].Uint64(BigInt.asUintN(64, v))
      }));
    }

    /** @returns {xdr.ScVal} the integer encoded with `ScValType = I256` */
  }, {
    key: "toI256",
    value: function toI256() {
      var v = this["int"].toBigInt();
      var hiHi64 = BigInt.asIntN(64, v >> 192n); // keep sign bit
      var hiLo64 = BigInt.asUintN(64, v >> 128n);
      var loHi64 = BigInt.asUintN(64, v >> 64n);
      var loLo64 = BigInt.asUintN(64, v);
      return _xdr["default"].ScVal.scvI256(new _xdr["default"].Int256Parts({
        hiHi: new _xdr["default"].Int64(hiHi64),
        hiLo: new _xdr["default"].Uint64(hiLo64),
        loHi: new _xdr["default"].Uint64(loHi64),
        loLo: new _xdr["default"].Uint64(loLo64)
      }));
    }

    /** @returns {xdr.ScVal} the integer encoded with `ScValType = U256` */
  }, {
    key: "toU256",
    value: function toU256() {
      var v = this["int"].toBigInt();
      var hiHi64 = BigInt.asUintN(64, v >> 192n); // encode sign bit
      var hiLo64 = BigInt.asUintN(64, v >> 128n);
      var loHi64 = BigInt.asUintN(64, v >> 64n);
      var loLo64 = BigInt.asUintN(64, v);
      return _xdr["default"].ScVal.scvU256(new _xdr["default"].UInt256Parts({
        hiHi: new _xdr["default"].Uint64(hiHi64),
        hiLo: new _xdr["default"].Uint64(hiLo64),
        loHi: new _xdr["default"].Uint64(loHi64),
        loLo: new _xdr["default"].Uint64(loLo64)
      }));
    }

    /** @returns {xdr.ScVal} the smallest interpretation of the stored value */
  }, {
    key: "toScVal",
    value: function toScVal() {
      switch (this.type) {
        case 'i64':
          return this.toI64();
        case 'i128':
          return this.toI128();
        case 'i256':
          return this.toI256();
        case 'u64':
          return this.toU64();
        case 'u128':
          return this.toU128();
        case 'u256':
          return this.toU256();
        default:
          throw TypeError("invalid type: ".concat(this.type));
      }
    }
  }, {
    key: "valueOf",
    value: function valueOf() {
      return this["int"].valueOf();
    }
  }, {
    key: "toString",
    value: function toString() {
      return this["int"].toString();
    }
  }, {
    key: "toJSON",
    value: function toJSON() {
      return {
        value: this.toBigInt().toString(),
        type: this.type
      };
    }
  }, {
    key: "_sizeCheck",
    value: function _sizeCheck(bits) {
      if (this["int"].size > bits) {
        throw RangeError("value too large for ".concat(bits, " bits (").concat(this.type, ")"));
      }
    }
  }], [{
    key: "isType",
    value: function isType(type) {
      switch (type) {
        case 'i64':
        case 'i128':
        case 'i256':
        case 'u64':
        case 'u128':
        case 'u256':
          return true;
        default:
          return false;
      }
    }

    /**
     * Convert the raw `ScValType` string (e.g. 'scvI128', generated by the XDR)
     * to a type description for {@link XdrLargeInt} construction (e.g. 'i128')
     *
     * @param {string} scvType  the `xdr.ScValType` as a string
     * @returns {string} a suitable equivalent type to construct this object
     */
  }, {
    key: "getType",
    value: function getType(scvType) {
      return scvType.slice(3).toLowerCase();
    }
  }]);
}();