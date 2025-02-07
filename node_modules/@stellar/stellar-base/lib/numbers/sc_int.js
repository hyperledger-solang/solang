"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.ScInt = void 0;
var _xdr_large_int = require("./xdr_large_int");
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _callSuper(t, o, e) { return o = _getPrototypeOf(o), _possibleConstructorReturn(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], _getPrototypeOf(t).constructor) : o.apply(t, e)); }
function _possibleConstructorReturn(t, e) { if (e && ("object" == _typeof(e) || "function" == typeof e)) return e; if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined"); return _assertThisInitialized(t); }
function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
function _inherits(t, e) { if ("function" != typeof e && null !== e) throw new TypeError("Super expression must either be null or a function"); t.prototype = Object.create(e && e.prototype, { constructor: { value: t, writable: !0, configurable: !0 } }), Object.defineProperty(t, "prototype", { writable: !1 }), e && _setPrototypeOf(t, e); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
/**
 * Provides an easier way to manipulate large numbers for Stellar operations.
 *
 * You can instantiate this "**s**mart **c**ontract integer" value either from
 * bigints, strings, or numbers (whole numbers, or this will throw).
 *
 * If you need to create a native BigInt from a list of integer "parts" (for
 * example, you have a series of encoded 32-bit integers that represent a larger
 * value), you can use the lower level abstraction {@link XdrLargeInt}. For
 * example, you could do `new XdrLargeInt('u128', bytes...).toBigInt()`.
 *
 * @example
 * import { xdr, ScInt, scValToBigInt } from "@stellar/stellar-base";
 *
 * // You have an ScVal from a contract and want to parse it into JS native.
 * const value = xdr.ScVal.fromXDR(someXdr, "base64");
 * const bigi = scValToBigInt(value); // grab it as a BigInt
 * let sci = new ScInt(bigi);
 *
 * sci.toNumber(); // gives native JS type (w/ size check)
 * sci.toBigInt(); // gives the native BigInt value
 * sci.toU64();    // gives ScValType-specific XDR constructs (with size checks)
 *
 * // You have a number and want to shove it into a contract.
 * sci = ScInt(0xdeadcafebabe);
 * sci.toBigInt() // returns 244838016400062n
 * sci.toNumber() // throws: too large
 *
 * // Pass any to e.g. a Contract.call(), conversion happens automatically
 * // regardless of the initial type.
 * const scValU128 = sci.toU128();
 * const scValI256 = sci.toI256();
 * const scValU64  = sci.toU64();
 *
 * // Lots of ways to initialize:
 * ScInt("123456789123456789")
 * ScInt(123456789123456789n);
 * ScInt(1n << 140n);
 * ScInt(-42);
 * ScInt(scValToBigInt(scValU128)); // from above
 *
 * // If you know the type ahead of time (accessing `.raw` is faster than
 * // conversions), you can specify the type directly (otherwise, it's
 * // interpreted from the numbers you pass in):
 * const i = ScInt(123456789n, { type: "u256" });
 *
 * // For example, you can use the underlying `sdk.U256` and convert it to an
 * // `xdr.ScVal` directly like so:
 * const scv = new xdr.ScVal.scvU256(i.raw);
 *
 * // Or reinterpret it as a different type (size permitting):
 * const scv = i.toI64();
 *
 * @param {number|bigint|string} value - a single, integer-like value which will
 *    be interpreted in the smallest appropriate XDR type supported by Stellar
 *    (64, 128, or 256 bit integer values). signed values are supported, though
 *    they are sanity-checked against `opts.type`. if you need 32-bit values,
 *    you can construct them directly without needing this wrapper, e.g.
 *    `xdr.ScVal.scvU32(1234)`.
 *
 * @param {object}  [opts] - an optional object controlling optional parameters
 * @param {string}  [opts.type] - force a specific data type. the type choices
 *    are: 'i64', 'u64', 'i128', 'u128', 'i256', and 'u256' (default: the
 *    smallest one that fits the `value`)
 *
 * @throws {RangeError} if the `value` is invalid (e.g. floating point), too
 *    large (i.e. exceeds a 256-bit value), or doesn't fit in the `opts.type`
 * @throws {TypeError} on missing parameters, or if the "signedness" of `opts`
 *    doesn't match input `value`, e.g. passing `{type: 'u64'}` yet passing -1n
 * @throws {SyntaxError} if a string `value` can't be parsed as a big integer
 */
var ScInt = exports.ScInt = /*#__PURE__*/function (_XdrLargeInt) {
  function ScInt(value, opts) {
    var _opts$type;
    _classCallCheck(this, ScInt);
    var signed = value < 0;
    var type = (_opts$type = opts === null || opts === void 0 ? void 0 : opts.type) !== null && _opts$type !== void 0 ? _opts$type : '';
    if (type.startsWith('u') && signed) {
      throw TypeError("specified type ".concat(opts.type, " yet negative (").concat(value, ")"));
    }

    // If unspecified, we make a best guess at the type based on the bit length
    // of the value, treating 64 as a minimum and 256 as a maximum.
    if (type === '') {
      type = signed ? 'i' : 'u';
      var bitlen = nearestBigIntSize(value);
      switch (bitlen) {
        case 64:
        case 128:
        case 256:
          type += bitlen.toString();
          break;
        default:
          throw RangeError("expected 64/128/256 bits for input (".concat(value, "), got ").concat(bitlen));
      }
    }
    return _callSuper(this, ScInt, [type, value]);
  }
  _inherits(ScInt, _XdrLargeInt);
  return _createClass(ScInt);
}(_xdr_large_int.XdrLargeInt);
function nearestBigIntSize(bigI) {
  var _find;
  // Note: Even though BigInt.toString(2) includes the negative sign for
  // negative values (???), the following is still accurate, because the
  // negative sign would be represented by a sign bit.
  var bitlen = bigI.toString(2).length;
  return (_find = [64, 128, 256].find(function (len) {
    return bitlen <= len;
  })) !== null && _find !== void 0 ? _find : bitlen;
}