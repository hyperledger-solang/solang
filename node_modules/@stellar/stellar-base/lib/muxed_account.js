"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.MuxedAccount = void 0;
var _xdr = _interopRequireDefault(require("./xdr"));
var _account = require("./account");
var _strkey = require("./strkey");
var _decode_encode_muxed_account = require("./util/decode_encode_muxed_account");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * Represents a muxed account for transactions and operations.
 *
 * A muxed (or *multiplexed*) account (defined rigorously in
 * [CAP-27](https://stellar.org/protocol/cap-27) and briefly in
 * [SEP-23](https://stellar.org/protocol/sep-23)) is one that resolves a single
 * Stellar `G...`` account to many different underlying IDs.
 *
 * For example, you may have a single Stellar address for accounting purposes:
 *   GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ
 *
 * Yet would like to use it for 4 different family members:
 *   1: MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJUAAAAAAAAAAAAGZFQ
 *   2: MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJUAAAAAAAAAAAALIWQ
 *   3: MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJUAAAAAAAAAAAAPYHQ
 *   4: MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJUAAAAAAAAAAAAQLQQ
 *
 * This object makes it easy to create muxed accounts from regular accounts,
 * duplicate them, get/set the underlying IDs, etc. without mucking around with
 * the raw XDR.
 *
 * Because muxed accounts are purely an off-chain convention, they all share the
 * sequence number tied to their underlying G... account. Thus, this object
 * *requires* an {@link Account} instance to be passed in, so that muxed
 * instances of an account can collectively modify the sequence number whenever
 * a muxed account is used as the source of a @{link Transaction} with {@link
 * TransactionBuilder}.
 *
 * @constructor
 *
 * @param {Account}   account - the @{link Account} instance representing the
 *                              underlying G... address
 * @param {string}    id      - a stringified uint64 value that represents the
 *                              ID of the muxed account
 *
 * @link https://developers.stellar.org/docs/glossary/muxed-accounts/
 */
var MuxedAccount = exports.MuxedAccount = /*#__PURE__*/function () {
  function MuxedAccount(baseAccount, id) {
    _classCallCheck(this, MuxedAccount);
    var accountId = baseAccount.accountId();
    if (!_strkey.StrKey.isValidEd25519PublicKey(accountId)) {
      throw new Error('accountId is invalid');
    }
    this.account = baseAccount;
    this._muxedXdr = (0, _decode_encode_muxed_account.encodeMuxedAccount)(accountId, id);
    this._mAddress = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(this._muxedXdr);
    this._id = id;
  }

  /**
   * Parses an M-address into a MuxedAccount object.
   *
   * @param  {string} mAddress    - an M-address to transform
   * @param  {string} sequenceNum - the sequence number of the underlying {@link
   *     Account}, to use for the underlying base account (@link
   *     MuxedAccount.baseAccount). If you're using the SDK, you can use
   *     `server.loadAccount` to fetch this if you don't know it.
   *
   * @return {MuxedAccount}
   */
  return _createClass(MuxedAccount, [{
    key: "baseAccount",
    value:
    /**
     * @return {Account} the underlying account object shared among all muxed
     *     accounts with this Stellar address
     */
    function baseAccount() {
      return this.account;
    }

    /**
     * @return {string} the M-address representing this account's (G-address, ID)
     */
  }, {
    key: "accountId",
    value: function accountId() {
      return this._mAddress;
    }
  }, {
    key: "id",
    value: function id() {
      return this._id;
    }
  }, {
    key: "setId",
    value: function setId(id) {
      if (typeof id !== 'string') {
        throw new Error('id should be a string representing a number (uint64)');
      }
      this._muxedXdr.med25519().id(_xdr["default"].Uint64.fromString(id));
      this._mAddress = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(this._muxedXdr);
      this._id = id;
      return this;
    }

    /**
     * Accesses the underlying account's sequence number.
     * @return {string}  strigified sequence number for the underlying account
     */
  }, {
    key: "sequenceNumber",
    value: function sequenceNumber() {
      return this.account.sequenceNumber();
    }

    /**
     * Increments the underlying account's sequence number by one.
     * @return {void}
     */
  }, {
    key: "incrementSequenceNumber",
    value: function incrementSequenceNumber() {
      return this.account.incrementSequenceNumber();
    }

    /**
     * @return {xdr.MuxedAccount} the XDR object representing this muxed account's
     *     G-address and uint64 ID
     */
  }, {
    key: "toXDRObject",
    value: function toXDRObject() {
      return this._muxedXdr;
    }
  }, {
    key: "equals",
    value: function equals(otherMuxedAccount) {
      return this.accountId() === otherMuxedAccount.accountId();
    }
  }], [{
    key: "fromAddress",
    value: function fromAddress(mAddress, sequenceNum) {
      var muxedAccount = (0, _decode_encode_muxed_account.decodeAddressToMuxedAccount)(mAddress);
      var gAddress = (0, _decode_encode_muxed_account.extractBaseAddress)(mAddress);
      var id = muxedAccount.med25519().id().toString();
      return new MuxedAccount(new _account.Account(gAddress, sequenceNum), id);
    }
  }]);
}();