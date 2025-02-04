"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.nativeToScVal = nativeToScVal;
exports.scValToNative = scValToNative;
var _xdr = _interopRequireDefault(require("./xdr"));
var _address = require("./address");
var _contract = require("./contract");
var _index = require("./numbers/index");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t["return"] && (u = t["return"](), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
/**
 * Attempts to convert native types into smart contract values
 * ({@link xdr.ScVal}).
 *
 * Provides conversions from smart contract XDR values ({@link xdr.ScVal}) to
 * native JavaScript types.
 *
 * The conversions are as follows:
 *
 *  - xdr.ScVal -> passthrough
 *  - null/undefined -> scvVoid
 *  - string -> scvString (a copy is made)
 *  - UintArray8 -> scvBytes (a copy is made)
 *  - boolean -> scvBool
 *
 *  - number/bigint -> the smallest possible XDR integer type that will fit the
 *    input value (if you want a specific type, use {@link ScInt})
 *
 *  - {@link Address} or {@link Contract} -> scvAddress (for contracts and
 *    public keys)
 *
 *  - Array<T> -> scvVec after attempting to convert each item of type `T` to an
 *    xdr.ScVal (recursively). note that all values must be the same type!
 *
 *  - object -> scvMap after attempting to convert each key and value to an
 *    xdr.ScVal (recursively). note that there is no restriction on types
 *    matching anywhere (unlike arrays)
 *
 * When passing an integer-like native value, you can also optionally specify a
 * type which will force a particular interpretation of that value.
 *
 * Note that not all type specifications are compatible with all `ScVal`s, e.g.
 * `toScVal("a string", {type: "i256"})` will throw.
 *
 * @param {any} val -       a native (or convertible) input value to wrap
 * @param {object} [opts] - an optional set of hints around the type of
 *    conversion you'd like to see
 * @param {string} [opts.type] - there is different behavior for different input
 *    types for `val`:
 *
 *     - when `val` is an integer-like type (i.e. number|bigint), this will be
 *       forwarded to {@link ScInt} or forced to be u32/i32.
 *
 *     - when `val` is an array type, this is forwarded to the recursion
 *
 *     - when `val` is an object type (key-value entries), this should be an
 *       object in which each key has a pair of types (to represent forced types
 *       for the key and the value), where `null` (or a missing entry) indicates
 *       the default interpretation(s) (refer to the examples, below)
 *
 *     - when `val` is a string type, this can be 'string' or 'symbol' to force
 *       a particular interpretation of `val`.
 *
 *     - when `val` is a bytes-like type, this can be 'string', 'symbol', or
 *       'bytes' to force a particular interpretation
 *
 *    As a simple example, `nativeToScVal("hello", {type: 'symbol'})` will
 *    return an `scvSymbol`, whereas without the type it would have been an
 *    `scvString`.
 *
 * @returns {xdr.ScVal} a wrapped, smart, XDR version of the input value
 * @throws {TypeError} if...
 *  - there are arrays with more than one type in them
 *  - there are values that do not have a sensible conversion (e.g. random XDR
 *    types, custom classes)
 *  - the type of the input object (or some inner value of said object) cannot
 *    be determined (via `typeof`)
 *  - the type you specified (via `opts.type`) is incompatible with the value
 *    you passed in (`val`), e.g. `nativeToScVal("a string", { type: 'i128' })`,
 *    though this does not apply for types that ignore `opts` (e.g. addresses).
 * @see scValToNative
 *
 * @example
 * nativeToScVal(1000);                   // gives ScValType === scvU64
 * nativeToScVal(1000n);                  // gives ScValType === scvU64
 * nativeToScVal(1n << 100n);             // gives ScValType === scvU128
 * nativeToScVal(1000, { type: 'u32' });  // gives ScValType === scvU32
 * nativeToScVal(1000, { type: 'i125' }); // gives ScValType === scvI256
 * nativeToScVal("a string");                     // gives ScValType === scvString
 * nativeToScVal("a string", { type: 'symbol' }); // gives scvSymbol
 * nativeToScVal(new Uint8Array(5));                      // scvBytes
 * nativeToScVal(new Uint8Array(5), { type: 'symbol' });  // scvSymbol
 * nativeToScVal(null); // scvVoid
 * nativeToScVal(true); // scvBool
 * nativeToScVal([1, 2, 3]);                    // gives scvVec with each element as scvU64
 * nativeToScVal([1, 2, 3], { type: 'i128' });  // scvVec<scvI128>
 * nativeToScVal({ 'hello': 1, 'world': [ true, false ] }, {
 *   type: {
 *     'hello': [ 'symbol', 'i128' ],
 *   }
 * })
 * // gives scvMap with entries: [
 * //     [ scvSymbol, scvI128 ],
 * //     [ scvString, scvArray<scvBool> ]
 * // ]
 *
 * @example
 * import {
 *   nativeToScVal,
 *   scValToNative,
 *   ScInt,
 *   xdr
 * } from '@stellar/stellar-base';
 *
 * let gigaMap = {
 *   bool: true,
 *   void: null,
 *   u32: xdr.ScVal.scvU32(1),
 *   i32: xdr.ScVal.scvI32(1),
 *   u64: 1n,
 *   i64: -1n,
 *   u128: new ScInt(1).toU128(),
 *   i128: new ScInt(1).toI128(),
 *   u256: new ScInt(1).toU256(),
 *   i256: new ScInt(1).toI256(),
 *   map: {
 *     arbitrary: 1n,
 *     nested: 'values',
 *     etc: false
 *   },
 *   vec: ['same', 'type', 'list'],
 * };
 *
 * // then, simply:
 * let scv = nativeToScVal(gigaMap);    // scv.switch() == xdr.ScValType.scvMap()
 *
 * // then...
 * someContract.call("method", scv);
 *
 * // Similarly, the inverse should work:
 * scValToNative(scv) == gigaMap;       // true
 */
function nativeToScVal(val) {
  var opts = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
  switch (_typeof(val)) {
    case 'object':
      {
        var _val$constructor$name, _val$constructor;
        if (val === null) {
          return _xdr["default"].ScVal.scvVoid();
        }
        if (val instanceof _xdr["default"].ScVal) {
          return val; // should we copy?
        }
        if (val instanceof _address.Address) {
          return val.toScVal();
        }
        if (val instanceof _contract.Contract) {
          return val.address().toScVal();
        }
        if (val instanceof Uint8Array || Buffer.isBuffer(val)) {
          var _opts$type;
          var copy = Uint8Array.from(val);
          switch ((_opts$type = opts === null || opts === void 0 ? void 0 : opts.type) !== null && _opts$type !== void 0 ? _opts$type : 'bytes') {
            case 'bytes':
              return _xdr["default"].ScVal.scvBytes(copy);
            case 'symbol':
              return _xdr["default"].ScVal.scvSymbol(copy);
            case 'string':
              return _xdr["default"].ScVal.scvString(copy);
            default:
              throw new TypeError("invalid type (".concat(opts.type, ") specified for bytes-like value"));
          }
        }
        if (Array.isArray(val)) {
          if (val.length > 0 && val.some(function (v) {
            return _typeof(v) !== _typeof(val[0]);
          })) {
            throw new TypeError("array values (".concat(val, ") must have the same type (types: ").concat(val.map(function (v) {
              return _typeof(v);
            }).join(','), ")"));
          }
          return _xdr["default"].ScVal.scvVec(val.map(function (v) {
            return nativeToScVal(v, opts);
          }));
        }
        if (((_val$constructor$name = (_val$constructor = val.constructor) === null || _val$constructor === void 0 ? void 0 : _val$constructor.name) !== null && _val$constructor$name !== void 0 ? _val$constructor$name : '') !== 'Object') {
          var _val$constructor2;
          throw new TypeError("cannot interpret ".concat((_val$constructor2 = val.constructor) === null || _val$constructor2 === void 0 ? void 0 : _val$constructor2.name, " value as ScVal (").concat(JSON.stringify(val), ")"));
        }
        return _xdr["default"].ScVal.scvMap(Object.entries(val)
        // The Soroban runtime expects maps to have their keys in sorted
        // order, so let's do that here as part of the conversion to prevent
        // confusing error messages on execution.
        .sort(function (_ref, _ref2) {
          var _ref3 = _slicedToArray(_ref, 1),
            key1 = _ref3[0];
          var _ref4 = _slicedToArray(_ref2, 1),
            key2 = _ref4[0];
          return key1.localeCompare(key2);
        }).map(function (_ref5) {
          var _k, _opts$type2;
          var _ref6 = _slicedToArray(_ref5, 2),
            k = _ref6[0],
            v = _ref6[1];
          // the type can be specified with an entry for the key and the value,
          // e.g. val = { 'hello': 1 } and opts.type = { hello: [ 'symbol',
          // 'u128' ]} or you can use `null` for the default interpretation
          var _ref7 = (_k = ((_opts$type2 = opts === null || opts === void 0 ? void 0 : opts.type) !== null && _opts$type2 !== void 0 ? _opts$type2 : {})[k]) !== null && _k !== void 0 ? _k : [null, null],
            _ref8 = _slicedToArray(_ref7, 2),
            keyType = _ref8[0],
            valType = _ref8[1];
          var keyOpts = keyType ? {
            type: keyType
          } : {};
          var valOpts = valType ? {
            type: valType
          } : {};
          return new _xdr["default"].ScMapEntry({
            key: nativeToScVal(k, keyOpts),
            val: nativeToScVal(v, valOpts)
          });
        }));
      }
    case 'number':
    case 'bigint':
      switch (opts === null || opts === void 0 ? void 0 : opts.type) {
        case 'u32':
          return _xdr["default"].ScVal.scvU32(val);
        case 'i32':
          return _xdr["default"].ScVal.scvI32(val);
        default:
          break;
      }
      return new _index.ScInt(val, {
        type: opts === null || opts === void 0 ? void 0 : opts.type
      }).toScVal();
    case 'string':
      {
        var _opts$type3;
        var optType = (_opts$type3 = opts === null || opts === void 0 ? void 0 : opts.type) !== null && _opts$type3 !== void 0 ? _opts$type3 : 'string';
        switch (optType) {
          case 'string':
            return _xdr["default"].ScVal.scvString(val);
          case 'symbol':
            return _xdr["default"].ScVal.scvSymbol(val);
          case 'address':
            return new _address.Address(val).toScVal();
          case 'u32':
            return _xdr["default"].ScVal.scvU32(parseInt(val, 10));
          case 'i32':
            return _xdr["default"].ScVal.scvI32(parseInt(val, 10));
          default:
            if (_index.XdrLargeInt.isType(optType)) {
              return new _index.XdrLargeInt(optType, val).toScVal();
            }
            throw new TypeError("invalid type (".concat(opts.type, ") specified for string value"));
        }
      }
    case 'boolean':
      return _xdr["default"].ScVal.scvBool(val);
    case 'undefined':
      return _xdr["default"].ScVal.scvVoid();
    case 'function':
      // FIXME: Is this too helpful?
      return nativeToScVal(val());
    default:
      throw new TypeError("failed to convert typeof ".concat(_typeof(val), " (").concat(val, ")"));
  }
}

/**
 * Given a smart contract value, attempt to convert it to a native type.
 * Possible conversions include:
 *
 *  - void -> `null`
 *  - u32, i32 -> `number`
 *  - u64, i64, u128, i128, u256, i256 -> `bigint`
 *  - vec -> `Array` of any of the above (via recursion)
 *  - map -> key-value object of any of the above (via recursion)
 *  - bool -> `boolean`
 *  - bytes -> `Uint8Array`
 *  - symbol -> `string`
 *  - string -> `string` IF the underlying buffer can be decoded as ascii/utf8,
 *              `Uint8Array` of the raw contents in any error case
 *
 * If no viable conversion can be determined, this just "unwraps" the smart
 * value to return its underlying XDR value.
 *
 * @param {xdr.ScVal} scv - the input smart contract value
 *
 * @returns {any}
 * @see nativeToScVal
 */
function scValToNative(scv) {
  var _scv$vec, _scv$map;
  // we use the verbose xdr.ScValType.<type>.value form here because it's faster
  // than string comparisons and the underlying constants never need to be
  // updated
  switch (scv["switch"]().value) {
    case _xdr["default"].ScValType.scvVoid().value:
      return null;

    // these can be converted to bigints directly
    case _xdr["default"].ScValType.scvU64().value:
    case _xdr["default"].ScValType.scvI64().value:
      return scv.value().toBigInt();

    // these can be parsed by internal abstractions note that this can also
    // handle the above two cases, but it's not as efficient (another
    // type-check, parsing, etc.)
    case _xdr["default"].ScValType.scvU128().value:
    case _xdr["default"].ScValType.scvI128().value:
    case _xdr["default"].ScValType.scvU256().value:
    case _xdr["default"].ScValType.scvI256().value:
      return (0, _index.scValToBigInt)(scv);
    case _xdr["default"].ScValType.scvVec().value:
      return ((_scv$vec = scv.vec()) !== null && _scv$vec !== void 0 ? _scv$vec : []).map(scValToNative);
    case _xdr["default"].ScValType.scvAddress().value:
      return _address.Address.fromScVal(scv).toString();
    case _xdr["default"].ScValType.scvMap().value:
      return Object.fromEntries(((_scv$map = scv.map()) !== null && _scv$map !== void 0 ? _scv$map : []).map(function (entry) {
        return [scValToNative(entry.key()), scValToNative(entry.val())];
      }));

    // these return the primitive type directly
    case _xdr["default"].ScValType.scvBool().value:
    case _xdr["default"].ScValType.scvU32().value:
    case _xdr["default"].ScValType.scvI32().value:
    case _xdr["default"].ScValType.scvBytes().value:
      return scv.value();

    // Symbols are limited to [a-zA-Z0-9_]+, so we can safely make ascii strings
    //
    // Strings, however, are "presented" as strings and we treat them as such
    // (in other words, string = bytes with a hint that it's text). If the user
    // encoded non-printable bytes in their string value, that's on them.
    //
    // Note that we assume a utf8 encoding (ascii-compatible). For other
    // encodings, you should probably use bytes anyway. If it cannot be decoded,
    // the raw bytes are returned.
    case _xdr["default"].ScValType.scvSymbol().value:
    case _xdr["default"].ScValType.scvString().value:
      {
        var v = scv.value(); // string|Buffer
        if (Buffer.isBuffer(v) || ArrayBuffer.isView(v)) {
          try {
            return new TextDecoder().decode(v);
          } catch (e) {
            return new Uint8Array(v.buffer); // copy of bytes
          }
        }
        return v; // string already
      }

    // these can be converted to bigint
    case _xdr["default"].ScValType.scvTimepoint().value:
    case _xdr["default"].ScValType.scvDuration().value:
      return new _xdr["default"].Uint64(scv.value()).toBigInt();
    case _xdr["default"].ScValType.scvError().value:
      switch (scv.error()["switch"]().value) {
        // Distinguish errors from the user contract.
        case _xdr["default"].ScErrorType.sceContract().value:
          return {
            type: 'contract',
            code: scv.error().contractCode()
          };
        default:
          {
            var err = scv.error();
            return {
              type: 'system',
              code: err.code().value,
              value: err.code().name
            };
          }
      }

    // in the fallthrough case, just return the underlying value directly
    default:
      return scv.value();
  }
}