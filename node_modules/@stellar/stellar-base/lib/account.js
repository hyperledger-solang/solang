"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Account = void 0;
var _bignumber = _interopRequireDefault(require("./util/bignumber"));
var _strkey = require("./strkey");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * Create a new Account object.
 *
 * `Account` represents a single account in the Stellar network and its sequence
 * number. Account tracks the sequence number as it is used by {@link
 * TransactionBuilder}. See
 * [Accounts](https://developers.stellar.org/docs/glossary/accounts/) for
 * more information about how accounts work in Stellar.
 *
 * @constructor
 *
 * @param {string} accountId - ID of the account (ex.
 *     `GB3KJPLFUYN5VL6R3GU3EGCGVCKFDSD7BEDX42HWG5BWFKB3KQGJJRMA`). If you
 *     provide a muxed account address, this will throw; use {@link
 *     MuxedAccount} instead.
 * @param {string} sequence  - current sequence number of the account
 */
var Account = exports.Account = /*#__PURE__*/function () {
  function Account(accountId, sequence) {
    _classCallCheck(this, Account);
    if (_strkey.StrKey.isValidMed25519PublicKey(accountId)) {
      throw new Error('accountId is an M-address; use MuxedAccount instead');
    }
    if (!_strkey.StrKey.isValidEd25519PublicKey(accountId)) {
      throw new Error('accountId is invalid');
    }
    if (!(typeof sequence === 'string')) {
      throw new Error('sequence must be of type string');
    }
    this._accountId = accountId;
    this.sequence = new _bignumber["default"](sequence);
  }

  /**
   * Returns Stellar account ID, ex.
   * `GB3KJPLFUYN5VL6R3GU3EGCGVCKFDSD7BEDX42HWG5BWFKB3KQGJJRMA`.
   * @returns {string}
   */
  return _createClass(Account, [{
    key: "accountId",
    value: function accountId() {
      return this._accountId;
    }

    /**
     * @returns {string}  sequence number for the account as a string
     */
  }, {
    key: "sequenceNumber",
    value: function sequenceNumber() {
      return this.sequence.toString();
    }

    /**
     * Increments sequence number in this object by one.
     * @returns {void}
     */
  }, {
    key: "incrementSequenceNumber",
    value: function incrementSequenceNumber() {
      this.sequence = this.sequence.plus(1);
    }
  }]);
}();