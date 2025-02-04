"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.MemoText = exports.MemoReturn = exports.MemoNone = exports.MemoID = exports.MemoHash = exports.Memo = void 0;
var _jsXdr = require("@stellar/js-xdr");
var _bignumber = _interopRequireDefault(require("./util/bignumber"));
var _xdr = _interopRequireDefault(require("./xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * Type of {@link Memo}.
 */
var MemoNone = exports.MemoNone = 'none';
/**
 * Type of {@link Memo}.
 */
var MemoID = exports.MemoID = 'id';
/**
 * Type of {@link Memo}.
 */
var MemoText = exports.MemoText = 'text';
/**
 * Type of {@link Memo}.
 */
var MemoHash = exports.MemoHash = 'hash';
/**
 * Type of {@link Memo}.
 */
var MemoReturn = exports.MemoReturn = 'return';

/**
 * `Memo` represents memos attached to transactions.
 *
 * @param {string} type - `MemoNone`, `MemoID`, `MemoText`, `MemoHash` or `MemoReturn`
 * @param {*} value - `string` for `MemoID`, `MemoText`, buffer of hex string for `MemoHash` or `MemoReturn`
 * @see [Transactions concept](https://developers.stellar.org/docs/glossary/transactions/)
 * @class Memo
 */
var Memo = exports.Memo = /*#__PURE__*/function () {
  function Memo(type) {
    var value = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : null;
    _classCallCheck(this, Memo);
    this._type = type;
    this._value = value;
    switch (this._type) {
      case MemoNone:
        break;
      case MemoID:
        Memo._validateIdValue(value);
        break;
      case MemoText:
        Memo._validateTextValue(value);
        break;
      case MemoHash:
      case MemoReturn:
        Memo._validateHashValue(value);
        // We want MemoHash and MemoReturn to have Buffer as a value
        if (typeof value === 'string') {
          this._value = Buffer.from(value, 'hex');
        }
        break;
      default:
        throw new Error('Invalid memo type');
    }
  }

  /**
   * Contains memo type: `MemoNone`, `MemoID`, `MemoText`, `MemoHash` or `MemoReturn`
   */
  return _createClass(Memo, [{
    key: "type",
    get: function get() {
      return this._type;
    },
    set: function set(type) {
      throw new Error('Memo is immutable');
    }

    /**
     * Contains memo value:
     * * `null` for `MemoNone`,
     * * `string` for `MemoID`,
     * * `Buffer` for `MemoText` after decoding using `fromXDRObject`, original value otherwise,
     * * `Buffer` for `MemoHash`, `MemoReturn`.
     */
  }, {
    key: "value",
    get: function get() {
      switch (this._type) {
        case MemoNone:
          return null;
        case MemoID:
        case MemoText:
          return this._value;
        case MemoHash:
        case MemoReturn:
          return Buffer.from(this._value);
        default:
          throw new Error('Invalid memo type');
      }
    },
    set: function set(value) {
      throw new Error('Memo is immutable');
    }
  }, {
    key: "toXDRObject",
    value:
    /**
     * Returns XDR memo object.
     * @returns {xdr.Memo}
     */
    function toXDRObject() {
      switch (this._type) {
        case MemoNone:
          return _xdr["default"].Memo.memoNone();
        case MemoID:
          return _xdr["default"].Memo.memoId(_jsXdr.UnsignedHyper.fromString(this._value));
        case MemoText:
          return _xdr["default"].Memo.memoText(this._value);
        case MemoHash:
          return _xdr["default"].Memo.memoHash(this._value);
        case MemoReturn:
          return _xdr["default"].Memo.memoReturn(this._value);
        default:
          return null;
      }
    }

    /**
     * Returns {@link Memo} from XDR memo object.
     * @param {xdr.Memo} object XDR memo object
     * @returns {Memo}
     */
  }], [{
    key: "_validateIdValue",
    value: function _validateIdValue(value) {
      var error = new Error("Expects a int64 as a string. Got ".concat(value));
      if (typeof value !== 'string') {
        throw error;
      }
      var number;
      try {
        number = new _bignumber["default"](value);
      } catch (e) {
        throw error;
      }

      // Infinity
      if (!number.isFinite()) {
        throw error;
      }

      // NaN
      if (number.isNaN()) {
        throw error;
      }
    }
  }, {
    key: "_validateTextValue",
    value: function _validateTextValue(value) {
      if (!_xdr["default"].Memo.armTypeForArm('text').isValid(value)) {
        throw new Error('Expects string, array or buffer, max 28 bytes');
      }
    }
  }, {
    key: "_validateHashValue",
    value: function _validateHashValue(value) {
      var error = new Error("Expects a 32 byte hash value or hex encoded string. Got ".concat(value));
      if (value === null || typeof value === 'undefined') {
        throw error;
      }
      var valueBuffer;
      if (typeof value === 'string') {
        if (!/^[0-9A-Fa-f]{64}$/g.test(value)) {
          throw error;
        }
        valueBuffer = Buffer.from(value, 'hex');
      } else if (Buffer.isBuffer(value)) {
        valueBuffer = Buffer.from(value);
      } else {
        throw error;
      }
      if (!valueBuffer.length || valueBuffer.length !== 32) {
        throw error;
      }
    }

    /**
     * Returns an empty memo (`MemoNone`).
     * @returns {Memo}
     */
  }, {
    key: "none",
    value: function none() {
      return new Memo(MemoNone);
    }

    /**
     * Creates and returns a `MemoText` memo.
     * @param {string} text - memo text
     * @returns {Memo}
     */
  }, {
    key: "text",
    value: function text(_text) {
      return new Memo(MemoText, _text);
    }

    /**
     * Creates and returns a `MemoID` memo.
     * @param {string} id - 64-bit number represented as a string
     * @returns {Memo}
     */
  }, {
    key: "id",
    value: function id(_id) {
      return new Memo(MemoID, _id);
    }

    /**
     * Creates and returns a `MemoHash` memo.
     * @param {array|string} hash - 32 byte hash or hex encoded string
     * @returns {Memo}
     */
  }, {
    key: "hash",
    value: function hash(_hash) {
      return new Memo(MemoHash, _hash);
    }

    /**
     * Creates and returns a `MemoReturn` memo.
     * @param {array|string} hash - 32 byte hash or hex encoded string
     * @returns {Memo}
     */
  }, {
    key: "return",
    value: function _return(hash) {
      return new Memo(MemoReturn, hash);
    }
  }, {
    key: "fromXDRObject",
    value: function fromXDRObject(object) {
      switch (object.arm()) {
        case 'id':
          return Memo.id(object.value().toString());
        case 'text':
          return Memo.text(object.value());
        case 'hash':
          return Memo.hash(object.value());
        case 'retHash':
          return Memo["return"](object.value());
        default:
          break;
      }
      if (typeof object.value() === 'undefined') {
        return Memo.none();
      }
      throw new Error('Unknown type');
    }
  }]);
}();