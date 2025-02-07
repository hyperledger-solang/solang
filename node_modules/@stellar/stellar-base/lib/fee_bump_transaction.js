"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.FeeBumpTransaction = void 0;
var _xdr = _interopRequireDefault(require("./xdr"));
var _hashing = require("./hashing");
var _transaction = require("./transaction");
var _transaction_base = require("./transaction_base");
var _decode_encode_muxed_account = require("./util/decode_encode_muxed_account");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _callSuper(t, o, e) { return o = _getPrototypeOf(o), _possibleConstructorReturn(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], _getPrototypeOf(t).constructor) : o.apply(t, e)); }
function _possibleConstructorReturn(t, e) { if (e && ("object" == _typeof(e) || "function" == typeof e)) return e; if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined"); return _assertThisInitialized(t); }
function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
function _inherits(t, e) { if ("function" != typeof e && null !== e) throw new TypeError("Super expression must either be null or a function"); t.prototype = Object.create(e && e.prototype, { constructor: { value: t, writable: !0, configurable: !0 } }), Object.defineProperty(t, "prototype", { writable: !1 }), e && _setPrototypeOf(t, e); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
/**
 * Use {@link TransactionBuilder.buildFeeBumpTransaction} to build a
 * FeeBumpTransaction object. If you have an object or base64-encoded string of
 * the transaction envelope XDR use {@link TransactionBuilder.fromXDR}.
 *
 * Once a {@link FeeBumpTransaction} has been created, its attributes and operations
 * should not be changed. You should only add signatures (using {@link FeeBumpTransaction#sign}) before
 * submitting to the network or forwarding on to additional signers.
 *
 * @param {string|xdr.TransactionEnvelope} envelope - transaction envelope
 *     object or base64 encoded string.
 * @param {string} networkPassphrase - passphrase of the target Stellar network
 *     (e.g. "Public Global Stellar Network ; September 2015").
 *
 * @extends TransactionBase
 */
var FeeBumpTransaction = exports.FeeBumpTransaction = /*#__PURE__*/function (_TransactionBase) {
  function FeeBumpTransaction(envelope, networkPassphrase) {
    var _this;
    _classCallCheck(this, FeeBumpTransaction);
    if (typeof envelope === 'string') {
      var buffer = Buffer.from(envelope, 'base64');
      envelope = _xdr["default"].TransactionEnvelope.fromXDR(buffer);
    }
    var envelopeType = envelope["switch"]();
    if (envelopeType !== _xdr["default"].EnvelopeType.envelopeTypeTxFeeBump()) {
      throw new Error("Invalid TransactionEnvelope: expected an envelopeTypeTxFeeBump but received an ".concat(envelopeType.name, "."));
    }
    var txEnvelope = envelope.value();
    var tx = txEnvelope.tx();
    var fee = tx.fee().toString();
    // clone signatures
    var signatures = (txEnvelope.signatures() || []).slice();
    _this = _callSuper(this, FeeBumpTransaction, [tx, signatures, fee, networkPassphrase]);
    var innerTxEnvelope = _xdr["default"].TransactionEnvelope.envelopeTypeTx(tx.innerTx().v1());
    _this._feeSource = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(_this.tx.feeSource());
    _this._innerTransaction = new _transaction.Transaction(innerTxEnvelope, networkPassphrase);
    return _this;
  }

  /**
   * @type {Transaction}
   * @readonly
   */
  _inherits(FeeBumpTransaction, _TransactionBase);
  return _createClass(FeeBumpTransaction, [{
    key: "innerTransaction",
    get: function get() {
      return this._innerTransaction;
    }

    /**
     * @type {Operation[]}
     * @readonly
     */
  }, {
    key: "operations",
    get: function get() {
      return this._innerTransaction.operations;
    }

    /**
     * @type {string}
     * @readonly
     */
  }, {
    key: "feeSource",
    get: function get() {
      return this._feeSource;
    }

    /**
     * Returns the "signature base" of this transaction, which is the value
     * that, when hashed, should be signed to create a signature that
     * validators on the Stellar Network will accept.
     *
     * It is composed of a 4 prefix bytes followed by the xdr-encoded form
     * of this transaction.
     * @returns {Buffer}
     */
  }, {
    key: "signatureBase",
    value: function signatureBase() {
      var taggedTransaction = new _xdr["default"].TransactionSignaturePayloadTaggedTransaction.envelopeTypeTxFeeBump(this.tx);
      var txSignature = new _xdr["default"].TransactionSignaturePayload({
        networkId: _xdr["default"].Hash.fromXDR((0, _hashing.hash)(this.networkPassphrase)),
        taggedTransaction: taggedTransaction
      });
      return txSignature.toXDR();
    }

    /**
     * To envelope returns a xdr.TransactionEnvelope which can be submitted to the network.
     * @returns {xdr.TransactionEnvelope}
     */
  }, {
    key: "toEnvelope",
    value: function toEnvelope() {
      var envelope = new _xdr["default"].FeeBumpTransactionEnvelope({
        tx: _xdr["default"].FeeBumpTransaction.fromXDR(this.tx.toXDR()),
        // make a copy of the tx
        signatures: this.signatures.slice() // make a copy of the signatures
      });
      return new _xdr["default"].TransactionEnvelope.envelopeTypeTxFeeBump(envelope);
    }
  }]);
}(_transaction_base.TransactionBase);