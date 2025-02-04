"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Transaction = void 0;
var _xdr = _interopRequireDefault(require("./xdr"));
var _hashing = require("./hashing");
var _strkey = require("./strkey");
var _operation = require("./operation");
var _memo = require("./memo");
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
 * Use {@link TransactionBuilder} to build a transaction object. If you have an
 * object or base64-encoded string of the transaction envelope XDR, use {@link
 * TransactionBuilder.fromXDR}.
 *
 * Once a Transaction has been created, its attributes and operations should not
 * be changed. You should only add signatures (using {@link Transaction#sign})
 * to a Transaction object before submitting to the network or forwarding on to
 * additional signers.
 *
 * @constructor
 *
 * @param {string|xdr.TransactionEnvelope} envelope - transaction envelope
 *     object or base64 encoded string
 * @param {string}  [networkPassphrase] - passphrase of the target stellar
 *     network (e.g. "Public Global Stellar Network ; September 2015")
 *
 * @extends TransactionBase
 */
var Transaction = exports.Transaction = /*#__PURE__*/function (_TransactionBase) {
  function Transaction(envelope, networkPassphrase) {
    var _this;
    _classCallCheck(this, Transaction);
    if (typeof envelope === 'string') {
      var buffer = Buffer.from(envelope, 'base64');
      envelope = _xdr["default"].TransactionEnvelope.fromXDR(buffer);
    }
    var envelopeType = envelope["switch"]();
    if (!(envelopeType === _xdr["default"].EnvelopeType.envelopeTypeTxV0() || envelopeType === _xdr["default"].EnvelopeType.envelopeTypeTx())) {
      throw new Error("Invalid TransactionEnvelope: expected an envelopeTypeTxV0 or envelopeTypeTx but received an ".concat(envelopeType.name, "."));
    }
    var txEnvelope = envelope.value();
    var tx = txEnvelope.tx();
    var fee = tx.fee().toString();
    var signatures = (txEnvelope.signatures() || []).slice();
    _this = _callSuper(this, Transaction, [tx, signatures, fee, networkPassphrase]);
    _this._envelopeType = envelopeType;
    _this._memo = tx.memo();
    _this._sequence = tx.seqNum().toString();
    switch (_this._envelopeType) {
      case _xdr["default"].EnvelopeType.envelopeTypeTxV0():
        _this._source = _strkey.StrKey.encodeEd25519PublicKey(_this.tx.sourceAccountEd25519());
        break;
      default:
        _this._source = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(_this.tx.sourceAccount());
        break;
    }
    var cond = null;
    var timeBounds = null;
    switch (_this._envelopeType) {
      case _xdr["default"].EnvelopeType.envelopeTypeTxV0():
        timeBounds = tx.timeBounds();
        break;
      case _xdr["default"].EnvelopeType.envelopeTypeTx():
        switch (tx.cond()["switch"]()) {
          case _xdr["default"].PreconditionType.precondTime():
            timeBounds = tx.cond().timeBounds();
            break;
          case _xdr["default"].PreconditionType.precondV2():
            cond = tx.cond().v2();
            timeBounds = cond.timeBounds();
            break;
          default:
            break;
        }
        break;
      default:
        break;
    }
    if (timeBounds) {
      _this._timeBounds = {
        minTime: timeBounds.minTime().toString(),
        maxTime: timeBounds.maxTime().toString()
      };
    }
    if (cond) {
      var ledgerBounds = cond.ledgerBounds();
      if (ledgerBounds) {
        _this._ledgerBounds = {
          minLedger: ledgerBounds.minLedger(),
          maxLedger: ledgerBounds.maxLedger()
        };
      }
      var minSeq = cond.minSeqNum();
      if (minSeq) {
        _this._minAccountSequence = minSeq.toString();
      }
      _this._minAccountSequenceAge = cond.minSeqAge();
      _this._minAccountSequenceLedgerGap = cond.minSeqLedgerGap();
      _this._extraSigners = cond.extraSigners();
    }
    var operations = tx.operations() || [];
    _this._operations = operations.map(function (op) {
      return _operation.Operation.fromXDRObject(op);
    });
    return _this;
  }

  /**
   * @type {object}
   * @property {string} 64 bit unix timestamp
   * @property {string} 64 bit unix timestamp
   * @readonly
   */
  _inherits(Transaction, _TransactionBase);
  return _createClass(Transaction, [{
    key: "timeBounds",
    get: function get() {
      return this._timeBounds;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * @type {object}
     * @property {number} minLedger - smallest ledger bound (uint32)
     * @property {number} maxLedger - largest ledger bound (or 0 for inf)
     * @readonly
     */
  }, {
    key: "ledgerBounds",
    get: function get() {
      return this._ledgerBounds;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * 64 bit account sequence
     * @readonly
     * @type {string}
     */
  }, {
    key: "minAccountSequence",
    get: function get() {
      return this._minAccountSequence;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * 64 bit number of seconds
     * @type {number}
     * @readonly
     */
  }, {
    key: "minAccountSequenceAge",
    get: function get() {
      return this._minAccountSequenceAge;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * 32 bit number of ledgers
     * @type {number}
     * @readonly
     */
  }, {
    key: "minAccountSequenceLedgerGap",
    get: function get() {
      return this._minAccountSequenceLedgerGap;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * array of extra signers ({@link StrKey}s)
     * @type {string[]}
     * @readonly
     */
  }, {
    key: "extraSigners",
    get: function get() {
      return this._extraSigners;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * @type {string}
     * @readonly
     */
  }, {
    key: "sequence",
    get: function get() {
      return this._sequence;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * @type {string}
     * @readonly
     */
  }, {
    key: "source",
    get: function get() {
      return this._source;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * @type {Array.<xdr.Operation>}
     * @readonly
     */
  }, {
    key: "operations",
    get: function get() {
      return this._operations;
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
    }

    /**
     * @type {string}
     * @readonly
     */
  }, {
    key: "memo",
    get: function get() {
      return _memo.Memo.fromXDRObject(this._memo);
    },
    set: function set(value) {
      throw new Error('Transaction is immutable');
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
      var tx = this.tx;

      // Backwards Compatibility: Use ENVELOPE_TYPE_TX to sign ENVELOPE_TYPE_TX_V0
      // we need a Transaction to generate the signature base
      if (this._envelopeType === _xdr["default"].EnvelopeType.envelopeTypeTxV0()) {
        tx = _xdr["default"].Transaction.fromXDR(Buffer.concat([
        // TransactionV0 is a transaction with the AccountID discriminant
        // stripped off, we need to put it back to build a valid transaction
        // which we can use to build a TransactionSignaturePayloadTaggedTransaction
        _xdr["default"].PublicKeyType.publicKeyTypeEd25519().toXDR(), tx.toXDR()]));
      }
      var taggedTransaction = new _xdr["default"].TransactionSignaturePayloadTaggedTransaction.envelopeTypeTx(tx);
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
      var rawTx = this.tx.toXDR();
      var signatures = this.signatures.slice(); // make a copy of the signatures

      var envelope;
      switch (this._envelopeType) {
        case _xdr["default"].EnvelopeType.envelopeTypeTxV0():
          envelope = new _xdr["default"].TransactionEnvelope.envelopeTypeTxV0(new _xdr["default"].TransactionV0Envelope({
            tx: _xdr["default"].TransactionV0.fromXDR(rawTx),
            // make a copy of tx
            signatures: signatures
          }));
          break;
        case _xdr["default"].EnvelopeType.envelopeTypeTx():
          envelope = new _xdr["default"].TransactionEnvelope.envelopeTypeTx(new _xdr["default"].TransactionV1Envelope({
            tx: _xdr["default"].Transaction.fromXDR(rawTx),
            // make a copy of tx
            signatures: signatures
          }));
          break;
        default:
          throw new Error("Invalid TransactionEnvelope: expected an envelopeTypeTxV0 or envelopeTypeTx but received an ".concat(this._envelopeType.name, "."));
      }
      return envelope;
    }

    /**
     * Calculate the claimable balance ID for an operation within the transaction.
     *
     * @param   {integer}  opIndex   the index of the CreateClaimableBalance op
     * @returns {string}   a hex string representing the claimable balance ID
     *
     * @throws {RangeError}   for invalid `opIndex` value
     * @throws {TypeError}    if op at `opIndex` is not `CreateClaimableBalance`
     * @throws for general XDR un/marshalling failures
     *
     * @see https://github.com/stellar/go/blob/d712346e61e288d450b0c08038c158f8848cc3e4/txnbuild/transaction.go#L392-L435
     *
     */
  }, {
    key: "getClaimableBalanceId",
    value: function getClaimableBalanceId(opIndex) {
      // Validate and then extract the operation from the transaction.
      if (!Number.isInteger(opIndex) || opIndex < 0 || opIndex >= this.operations.length) {
        throw new RangeError('invalid operation index');
      }
      var op = this.operations[opIndex];
      try {
        op = _operation.Operation.createClaimableBalance(op);
      } catch (err) {
        throw new TypeError("expected createClaimableBalance, got ".concat(op.type, ": ").concat(err));
      }

      // Always use the transaction's *unmuxed* source.
      var account = _strkey.StrKey.decodeEd25519PublicKey((0, _decode_encode_muxed_account.extractBaseAddress)(this.source));
      var operationId = _xdr["default"].HashIdPreimage.envelopeTypeOpId(new _xdr["default"].HashIdPreimageOperationId({
        sourceAccount: _xdr["default"].AccountId.publicKeyTypeEd25519(account),
        seqNum: _xdr["default"].SequenceNumber.fromString(this.sequence),
        opNum: opIndex
      }));
      var opIdHash = (0, _hashing.hash)(operationId.toXDR('raw'));
      var balanceId = _xdr["default"].ClaimableBalanceId.claimableBalanceIdTypeV0(opIdHash);
      return balanceId.toXDR('hex');
    }
  }]);
}(_transaction_base.TransactionBase);