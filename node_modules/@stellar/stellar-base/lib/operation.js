"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Operation = exports.AuthRevocableFlag = exports.AuthRequiredFlag = exports.AuthImmutableFlag = exports.AuthClawbackEnabledFlag = void 0;
var _jsXdr = require("@stellar/js-xdr");
var _bignumber = _interopRequireDefault(require("./util/bignumber"));
var _util = require("./util/util");
var _continued_fraction = require("./util/continued_fraction");
var _asset = require("./asset");
var _liquidity_pool_asset = require("./liquidity_pool_asset");
var _claimant = require("./claimant");
var _strkey = require("./strkey");
var _liquidity_pool_id = require("./liquidity_pool_id");
var _xdr = _interopRequireDefault(require("./xdr"));
var ops = _interopRequireWildcard(require("./operations"));
var _decode_encode_muxed_account = require("./util/decode_encode_muxed_account");
function _getRequireWildcardCache(e) { if ("function" != typeof WeakMap) return null; var r = new WeakMap(), t = new WeakMap(); return (_getRequireWildcardCache = function _getRequireWildcardCache(e) { return e ? t : r; })(e); }
function _interopRequireWildcard(e, r) { if (!r && e && e.__esModule) return e; if (null === e || "object" != _typeof(e) && "function" != typeof e) return { "default": e }; var t = _getRequireWildcardCache(r); if (t && t.has(e)) return t.get(e); var n = { __proto__: null }, a = Object.defineProperty && Object.getOwnPropertyDescriptor; for (var u in e) if ("default" !== u && {}.hasOwnProperty.call(e, u)) { var i = a ? Object.getOwnPropertyDescriptor(e, u) : null; i && (i.get || i.set) ? Object.defineProperty(n, u, i) : n[u] = e[u]; } return n["default"] = e, t && t.set(e, n), n; }
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); } /* eslint-disable no-bitwise */
var ONE = 10000000;
var MAX_INT64 = '9223372036854775807';

/**
 * When set using `{@link Operation.setOptions}` option, requires the issuing
 * account to give other accounts permission before they can hold the issuing
 * account’s credit.
 *
 * @constant
 * @see [Account flags](https://developers.stellar.org/docs/glossary/accounts/#flags)
 */
var AuthRequiredFlag = exports.AuthRequiredFlag = 1 << 0;
/**
 * When set using `{@link Operation.setOptions}` option, allows the issuing
 * account to revoke its credit held by other accounts.
 *
 * @constant
 * @see [Account flags](https://developers.stellar.org/docs/glossary/accounts/#flags)
 */
var AuthRevocableFlag = exports.AuthRevocableFlag = 1 << 1;
/**
 * When set using `{@link Operation.setOptions}` option, then none of the
 * authorization flags can be set and the account can never be deleted.
 *
 * @constant
 * @see [Account flags](https://developers.stellar.org/docs/glossary/accounts/#flags)
 */
var AuthImmutableFlag = exports.AuthImmutableFlag = 1 << 2;

/**
 * When set using `{@link Operation.setOptions}` option, then any trustlines
 * created by this account can have a ClawbackOp operation submitted for the
 * corresponding asset.
 *
 * @constant
 * @see [Account flags](https://developers.stellar.org/docs/glossary/accounts/#flags)
 */
var AuthClawbackEnabledFlag = exports.AuthClawbackEnabledFlag = 1 << 3;

/**
 * `Operation` class represents
 * [operations](https://developers.stellar.org/docs/glossary/operations/) in
 * Stellar network.
 *
 * Use one of static methods to create operations:
 * * `{@link Operation.createAccount}`
 * * `{@link Operation.payment}`
 * * `{@link Operation.pathPaymentStrictReceive}`
 * * `{@link Operation.pathPaymentStrictSend}`
 * * `{@link Operation.manageSellOffer}`
 * * `{@link Operation.manageBuyOffer}`
 * * `{@link Operation.createPassiveSellOffer}`
 * * `{@link Operation.setOptions}`
 * * `{@link Operation.changeTrust}`
 * * `{@link Operation.allowTrust}`
 * * `{@link Operation.accountMerge}`
 * * `{@link Operation.inflation}`
 * * `{@link Operation.manageData}`
 * * `{@link Operation.bumpSequence}`
 * * `{@link Operation.createClaimableBalance}`
 * * `{@link Operation.claimClaimableBalance}`
 * * `{@link Operation.beginSponsoringFutureReserves}`
 * * `{@link Operation.endSponsoringFutureReserves}`
 * * `{@link Operation.revokeAccountSponsorship}`
 * * `{@link Operation.revokeTrustlineSponsorship}`
 * * `{@link Operation.revokeOfferSponsorship}`
 * * `{@link Operation.revokeDataSponsorship}`
 * * `{@link Operation.revokeClaimableBalanceSponsorship}`
 * * `{@link Operation.revokeLiquidityPoolSponsorship}`
 * * `{@link Operation.revokeSignerSponsorship}`
 * * `{@link Operation.clawback}`
 * * `{@link Operation.clawbackClaimableBalance}`
 * * `{@link Operation.setTrustLineFlags}`
 * * `{@link Operation.liquidityPoolDeposit}`
 * * `{@link Operation.liquidityPoolWithdraw}`
 * * `{@link Operation.invokeHostFunction}`, which has the following additional
 *   "pseudo-operations" that make building host functions easier:
 *   - `{@link Operation.createStellarAssetContract}`
 *   - `{@link Operation.invokeContractFunction}`
 *   - `{@link Operation.createCustomContract}`
 *   - `{@link Operation.uploadContractWasm}`
 * * `{@link Operation.extendFootprintTtlOp}`
 * * `{@link Operation.restoreFootprint}`
 *
 * @class Operation
 */
var Operation = exports.Operation = /*#__PURE__*/function () {
  function Operation() {
    _classCallCheck(this, Operation);
  }
  return _createClass(Operation, null, [{
    key: "setSourceAccount",
    value: function setSourceAccount(opAttributes, opts) {
      if (opts.source) {
        try {
          opAttributes.sourceAccount = (0, _decode_encode_muxed_account.decodeAddressToMuxedAccount)(opts.source);
        } catch (e) {
          throw new Error('Source address is invalid');
        }
      }
    }

    /**
     * Deconstructs the raw XDR operation object into the structured object that
     * was used to create the operation (i.e. the `opts` parameter to most ops).
     *
     * @param {xdr.Operation}   operation - An XDR Operation.
     * @return {Operation}
     */
  }, {
    key: "fromXDRObject",
    value: function fromXDRObject(operation) {
      var result = {};
      if (operation.sourceAccount()) {
        result.source = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(operation.sourceAccount());
      }
      var attrs = operation.body().value();
      var operationName = operation.body()["switch"]().name;
      switch (operationName) {
        case 'createAccount':
          {
            result.type = 'createAccount';
            result.destination = accountIdtoAddress(attrs.destination());
            result.startingBalance = this._fromXDRAmount(attrs.startingBalance());
            break;
          }
        case 'payment':
          {
            result.type = 'payment';
            result.destination = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(attrs.destination());
            result.asset = _asset.Asset.fromOperation(attrs.asset());
            result.amount = this._fromXDRAmount(attrs.amount());
            break;
          }
        case 'pathPaymentStrictReceive':
          {
            result.type = 'pathPaymentStrictReceive';
            result.sendAsset = _asset.Asset.fromOperation(attrs.sendAsset());
            result.sendMax = this._fromXDRAmount(attrs.sendMax());
            result.destination = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(attrs.destination());
            result.destAsset = _asset.Asset.fromOperation(attrs.destAsset());
            result.destAmount = this._fromXDRAmount(attrs.destAmount());
            result.path = [];
            var path = attrs.path();

            // note that Object.values isn't supported by node 6!
            Object.keys(path).forEach(function (pathKey) {
              result.path.push(_asset.Asset.fromOperation(path[pathKey]));
            });
            break;
          }
        case 'pathPaymentStrictSend':
          {
            result.type = 'pathPaymentStrictSend';
            result.sendAsset = _asset.Asset.fromOperation(attrs.sendAsset());
            result.sendAmount = this._fromXDRAmount(attrs.sendAmount());
            result.destination = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(attrs.destination());
            result.destAsset = _asset.Asset.fromOperation(attrs.destAsset());
            result.destMin = this._fromXDRAmount(attrs.destMin());
            result.path = [];
            var _path = attrs.path();

            // note that Object.values isn't supported by node 6!
            Object.keys(_path).forEach(function (pathKey) {
              result.path.push(_asset.Asset.fromOperation(_path[pathKey]));
            });
            break;
          }
        case 'changeTrust':
          {
            result.type = 'changeTrust';
            switch (attrs.line()["switch"]()) {
              case _xdr["default"].AssetType.assetTypePoolShare():
                result.line = _liquidity_pool_asset.LiquidityPoolAsset.fromOperation(attrs.line());
                break;
              default:
                result.line = _asset.Asset.fromOperation(attrs.line());
                break;
            }
            result.limit = this._fromXDRAmount(attrs.limit());
            break;
          }
        case 'allowTrust':
          {
            result.type = 'allowTrust';
            result.trustor = accountIdtoAddress(attrs.trustor());
            result.assetCode = attrs.asset().value().toString();
            result.assetCode = (0, _util.trimEnd)(result.assetCode, '\0');
            result.authorize = attrs.authorize();
            break;
          }
        case 'setOptions':
          {
            result.type = 'setOptions';
            if (attrs.inflationDest()) {
              result.inflationDest = accountIdtoAddress(attrs.inflationDest());
            }
            result.clearFlags = attrs.clearFlags();
            result.setFlags = attrs.setFlags();
            result.masterWeight = attrs.masterWeight();
            result.lowThreshold = attrs.lowThreshold();
            result.medThreshold = attrs.medThreshold();
            result.highThreshold = attrs.highThreshold();
            // home_domain is checked by iscntrl in stellar-core
            result.homeDomain = attrs.homeDomain() !== undefined ? attrs.homeDomain().toString('ascii') : undefined;
            if (attrs.signer()) {
              var signer = {};
              var arm = attrs.signer().key().arm();
              if (arm === 'ed25519') {
                signer.ed25519PublicKey = accountIdtoAddress(attrs.signer().key());
              } else if (arm === 'preAuthTx') {
                signer.preAuthTx = attrs.signer().key().preAuthTx();
              } else if (arm === 'hashX') {
                signer.sha256Hash = attrs.signer().key().hashX();
              } else if (arm === 'ed25519SignedPayload') {
                var signedPayload = attrs.signer().key().ed25519SignedPayload();
                signer.ed25519SignedPayload = _strkey.StrKey.encodeSignedPayload(signedPayload.toXDR());
              }
              signer.weight = attrs.signer().weight();
              result.signer = signer;
            }
            break;
          }
        // the next case intentionally falls through!
        case 'manageOffer':
        case 'manageSellOffer':
          {
            result.type = 'manageSellOffer';
            result.selling = _asset.Asset.fromOperation(attrs.selling());
            result.buying = _asset.Asset.fromOperation(attrs.buying());
            result.amount = this._fromXDRAmount(attrs.amount());
            result.price = this._fromXDRPrice(attrs.price());
            result.offerId = attrs.offerId().toString();
            break;
          }
        case 'manageBuyOffer':
          {
            result.type = 'manageBuyOffer';
            result.selling = _asset.Asset.fromOperation(attrs.selling());
            result.buying = _asset.Asset.fromOperation(attrs.buying());
            result.buyAmount = this._fromXDRAmount(attrs.buyAmount());
            result.price = this._fromXDRPrice(attrs.price());
            result.offerId = attrs.offerId().toString();
            break;
          }
        // the next case intentionally falls through!
        case 'createPassiveOffer':
        case 'createPassiveSellOffer':
          {
            result.type = 'createPassiveSellOffer';
            result.selling = _asset.Asset.fromOperation(attrs.selling());
            result.buying = _asset.Asset.fromOperation(attrs.buying());
            result.amount = this._fromXDRAmount(attrs.amount());
            result.price = this._fromXDRPrice(attrs.price());
            break;
          }
        case 'accountMerge':
          {
            result.type = 'accountMerge';
            result.destination = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(attrs);
            break;
          }
        case 'manageData':
          {
            result.type = 'manageData';
            // manage_data.name is checked by iscntrl in stellar-core
            result.name = attrs.dataName().toString('ascii');
            result.value = attrs.dataValue();
            break;
          }
        case 'inflation':
          {
            result.type = 'inflation';
            break;
          }
        case 'bumpSequence':
          {
            result.type = 'bumpSequence';
            result.bumpTo = attrs.bumpTo().toString();
            break;
          }
        case 'createClaimableBalance':
          {
            result.type = 'createClaimableBalance';
            result.asset = _asset.Asset.fromOperation(attrs.asset());
            result.amount = this._fromXDRAmount(attrs.amount());
            result.claimants = [];
            attrs.claimants().forEach(function (claimant) {
              result.claimants.push(_claimant.Claimant.fromXDR(claimant));
            });
            break;
          }
        case 'claimClaimableBalance':
          {
            result.type = 'claimClaimableBalance';
            result.balanceId = attrs.toXDR('hex');
            break;
          }
        case 'beginSponsoringFutureReserves':
          {
            result.type = 'beginSponsoringFutureReserves';
            result.sponsoredId = accountIdtoAddress(attrs.sponsoredId());
            break;
          }
        case 'endSponsoringFutureReserves':
          {
            result.type = 'endSponsoringFutureReserves';
            break;
          }
        case 'revokeSponsorship':
          {
            extractRevokeSponshipDetails(attrs, result);
            break;
          }
        case 'clawback':
          {
            result.type = 'clawback';
            result.amount = this._fromXDRAmount(attrs.amount());
            result.from = (0, _decode_encode_muxed_account.encodeMuxedAccountToAddress)(attrs.from());
            result.asset = _asset.Asset.fromOperation(attrs.asset());
            break;
          }
        case 'clawbackClaimableBalance':
          {
            result.type = 'clawbackClaimableBalance';
            result.balanceId = attrs.toXDR('hex');
            break;
          }
        case 'setTrustLineFlags':
          {
            result.type = 'setTrustLineFlags';
            result.asset = _asset.Asset.fromOperation(attrs.asset());
            result.trustor = accountIdtoAddress(attrs.trustor());

            // Convert from the integer-bitwised flag into a sensible object that
            // indicates true/false for each flag that's on/off.
            var clears = attrs.clearFlags();
            var sets = attrs.setFlags();
            var mapping = {
              authorized: _xdr["default"].TrustLineFlags.authorizedFlag(),
              authorizedToMaintainLiabilities: _xdr["default"].TrustLineFlags.authorizedToMaintainLiabilitiesFlag(),
              clawbackEnabled: _xdr["default"].TrustLineFlags.trustlineClawbackEnabledFlag()
            };
            var getFlagValue = function getFlagValue(key) {
              var bit = mapping[key].value;
              if (sets & bit) {
                return true;
              }
              if (clears & bit) {
                return false;
              }
              return undefined;
            };
            result.flags = {};
            Object.keys(mapping).forEach(function (flagName) {
              result.flags[flagName] = getFlagValue(flagName);
            });
            break;
          }
        case 'liquidityPoolDeposit':
          {
            result.type = 'liquidityPoolDeposit';
            result.liquidityPoolId = attrs.liquidityPoolId().toString('hex');
            result.maxAmountA = this._fromXDRAmount(attrs.maxAmountA());
            result.maxAmountB = this._fromXDRAmount(attrs.maxAmountB());
            result.minPrice = this._fromXDRPrice(attrs.minPrice());
            result.maxPrice = this._fromXDRPrice(attrs.maxPrice());
            break;
          }
        case 'liquidityPoolWithdraw':
          {
            result.type = 'liquidityPoolWithdraw';
            result.liquidityPoolId = attrs.liquidityPoolId().toString('hex');
            result.amount = this._fromXDRAmount(attrs.amount());
            result.minAmountA = this._fromXDRAmount(attrs.minAmountA());
            result.minAmountB = this._fromXDRAmount(attrs.minAmountB());
            break;
          }
        case 'invokeHostFunction':
          {
            var _attrs$auth;
            result.type = 'invokeHostFunction';
            result.func = attrs.hostFunction();
            result.auth = (_attrs$auth = attrs.auth()) !== null && _attrs$auth !== void 0 ? _attrs$auth : [];
            break;
          }
        case 'extendFootprintTtl':
          {
            result.type = 'extendFootprintTtl';
            result.extendTo = attrs.extendTo();
            break;
          }
        case 'restoreFootprint':
          {
            result.type = 'restoreFootprint';
            break;
          }
        default:
          {
            throw new Error("Unknown operation: ".concat(operationName));
          }
      }
      return result;
    }

    /**
     * Validates that a given amount is possible for a Stellar asset.
     *
     * Specifically, this means that the amount is well, a valid number, but also
     * that it is within the int64 range and has no more than 7 decimal levels of
     * precision.
     *
     * Note that while smart contracts allow larger amounts, this is oriented
     * towards validating the standard Stellar operations.
     *
     * @param {string}  value       the amount to validate
     * @param {boolean} allowZero   optionally, whether or not zero is valid (default: no)
     *
     * @returns {boolean}
     */
  }, {
    key: "isValidAmount",
    value: function isValidAmount(value) {
      var allowZero = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : false;
      if (typeof value !== 'string') {
        return false;
      }
      var amount;
      try {
        amount = new _bignumber["default"](value);
      } catch (e) {
        return false;
      }
      if (
      // == 0
      !allowZero && amount.isZero() ||
      // < 0
      amount.isNegative() ||
      // > Max value
      amount.times(ONE).gt(new _bignumber["default"](MAX_INT64).toString()) ||
      // Decimal places (max 7)
      amount.decimalPlaces() > 7 ||
      // NaN or Infinity
      amount.isNaN() || !amount.isFinite()) {
        return false;
      }
      return true;
    }
  }, {
    key: "constructAmountRequirementsError",
    value: function constructAmountRequirementsError(arg) {
      return "".concat(arg, " argument must be of type String, represent a positive number and have at most 7 digits after the decimal");
    }

    /**
     * Returns value converted to uint32 value or undefined.
     * If `value` is not `Number`, `String` or `Undefined` then throws an error.
     * Used in {@link Operation.setOptions}.
     * @private
     * @param {string} name Name of the property (used in error message only)
     * @param {*} value Value to check
     * @param {function(value, name)} isValidFunction Function to check other constraints (the argument will be a `Number`)
     * @returns {undefined|Number}
     */
  }, {
    key: "_checkUnsignedIntValue",
    value: function _checkUnsignedIntValue(name, value) {
      var isValidFunction = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : null;
      if (typeof value === 'undefined') {
        return undefined;
      }
      if (typeof value === 'string') {
        value = parseFloat(value);
      }
      switch (true) {
        case typeof value !== 'number' || !Number.isFinite(value) || value % 1 !== 0:
          throw new Error("".concat(name, " value is invalid"));
        case value < 0:
          throw new Error("".concat(name, " value must be unsigned"));
        case !isValidFunction || isValidFunction && isValidFunction(value, name):
          return value;
        default:
          throw new Error("".concat(name, " value is invalid"));
      }
    }
    /**
     * @private
     * @param {string|BigNumber} value Value
     * @returns {Hyper} XDR amount
     */
  }, {
    key: "_toXDRAmount",
    value: function _toXDRAmount(value) {
      var amount = new _bignumber["default"](value).times(ONE);
      return _jsXdr.Hyper.fromString(amount.toString());
    }

    /**
     * @private
     * @param {string|BigNumber} value XDR amount
     * @returns {BigNumber} Number
     */
  }, {
    key: "_fromXDRAmount",
    value: function _fromXDRAmount(value) {
      return new _bignumber["default"](value).div(ONE).toFixed(7);
    }

    /**
     * @private
     * @param {object} price Price object
     * @param {function} price.n numerator function that returns a value
     * @param {function} price.d denominator function that returns a value
     * @returns {BigNumber} Big string
     */
  }, {
    key: "_fromXDRPrice",
    value: function _fromXDRPrice(price) {
      var n = new _bignumber["default"](price.n());
      return n.div(new _bignumber["default"](price.d())).toString();
    }

    /**
     * @private
     * @param {object} price Price object
     * @param {function} price.n numerator function that returns a value
     * @param {function} price.d denominator function that returns a value
     * @returns {object} XDR price object
     */
  }, {
    key: "_toXDRPrice",
    value: function _toXDRPrice(price) {
      var xdrObject;
      if (price.n && price.d) {
        xdrObject = new _xdr["default"].Price(price);
      } else {
        var approx = (0, _continued_fraction.best_r)(price);
        xdrObject = new _xdr["default"].Price({
          n: parseInt(approx[0], 10),
          d: parseInt(approx[1], 10)
        });
      }
      if (xdrObject.n() < 0 || xdrObject.d() < 0) {
        throw new Error('price must be positive');
      }
      return xdrObject;
    }
  }]);
}();
function extractRevokeSponshipDetails(attrs, result) {
  switch (attrs["switch"]().name) {
    case 'revokeSponsorshipLedgerEntry':
      {
        var ledgerKey = attrs.ledgerKey();
        switch (ledgerKey["switch"]().name) {
          case _xdr["default"].LedgerEntryType.account().name:
            {
              result.type = 'revokeAccountSponsorship';
              result.account = accountIdtoAddress(ledgerKey.account().accountId());
              break;
            }
          case _xdr["default"].LedgerEntryType.trustline().name:
            {
              result.type = 'revokeTrustlineSponsorship';
              result.account = accountIdtoAddress(ledgerKey.trustLine().accountId());
              var xdrAsset = ledgerKey.trustLine().asset();
              switch (xdrAsset["switch"]()) {
                case _xdr["default"].AssetType.assetTypePoolShare():
                  result.asset = _liquidity_pool_id.LiquidityPoolId.fromOperation(xdrAsset);
                  break;
                default:
                  result.asset = _asset.Asset.fromOperation(xdrAsset);
                  break;
              }
              break;
            }
          case _xdr["default"].LedgerEntryType.offer().name:
            {
              result.type = 'revokeOfferSponsorship';
              result.seller = accountIdtoAddress(ledgerKey.offer().sellerId());
              result.offerId = ledgerKey.offer().offerId().toString();
              break;
            }
          case _xdr["default"].LedgerEntryType.data().name:
            {
              result.type = 'revokeDataSponsorship';
              result.account = accountIdtoAddress(ledgerKey.data().accountId());
              result.name = ledgerKey.data().dataName().toString('ascii');
              break;
            }
          case _xdr["default"].LedgerEntryType.claimableBalance().name:
            {
              result.type = 'revokeClaimableBalanceSponsorship';
              result.balanceId = ledgerKey.claimableBalance().balanceId().toXDR('hex');
              break;
            }
          case _xdr["default"].LedgerEntryType.liquidityPool().name:
            {
              result.type = 'revokeLiquidityPoolSponsorship';
              result.liquidityPoolId = ledgerKey.liquidityPool().liquidityPoolId().toString('hex');
              break;
            }
          default:
            {
              throw new Error("Unknown ledgerKey: ".concat(attrs["switch"]().name));
            }
        }
        break;
      }
    case 'revokeSponsorshipSigner':
      {
        result.type = 'revokeSignerSponsorship';
        result.account = accountIdtoAddress(attrs.signer().accountId());
        result.signer = convertXDRSignerKeyToObject(attrs.signer().signerKey());
        break;
      }
    default:
      {
        throw new Error("Unknown revokeSponsorship: ".concat(attrs["switch"]().name));
      }
  }
}
function convertXDRSignerKeyToObject(signerKey) {
  var attrs = {};
  switch (signerKey["switch"]().name) {
    case _xdr["default"].SignerKeyType.signerKeyTypeEd25519().name:
      {
        attrs.ed25519PublicKey = _strkey.StrKey.encodeEd25519PublicKey(signerKey.ed25519());
        break;
      }
    case _xdr["default"].SignerKeyType.signerKeyTypePreAuthTx().name:
      {
        attrs.preAuthTx = signerKey.preAuthTx().toString('hex');
        break;
      }
    case _xdr["default"].SignerKeyType.signerKeyTypeHashX().name:
      {
        attrs.sha256Hash = signerKey.hashX().toString('hex');
        break;
      }
    default:
      {
        throw new Error("Unknown signerKey: ".concat(signerKey["switch"]().name));
      }
  }
  return attrs;
}
function accountIdtoAddress(accountId) {
  return _strkey.StrKey.encodeEd25519PublicKey(accountId.ed25519());
}

// Attach all imported operations as static methods on the Operation class
Operation.accountMerge = ops.accountMerge;
Operation.allowTrust = ops.allowTrust;
Operation.bumpSequence = ops.bumpSequence;
Operation.changeTrust = ops.changeTrust;
Operation.createAccount = ops.createAccount;
Operation.createClaimableBalance = ops.createClaimableBalance;
Operation.claimClaimableBalance = ops.claimClaimableBalance;
Operation.clawbackClaimableBalance = ops.clawbackClaimableBalance;
Operation.createPassiveSellOffer = ops.createPassiveSellOffer;
Operation.inflation = ops.inflation;
Operation.manageData = ops.manageData;
Operation.manageSellOffer = ops.manageSellOffer;
Operation.manageBuyOffer = ops.manageBuyOffer;
Operation.pathPaymentStrictReceive = ops.pathPaymentStrictReceive;
Operation.pathPaymentStrictSend = ops.pathPaymentStrictSend;
Operation.payment = ops.payment;
Operation.setOptions = ops.setOptions;
Operation.beginSponsoringFutureReserves = ops.beginSponsoringFutureReserves;
Operation.endSponsoringFutureReserves = ops.endSponsoringFutureReserves;
Operation.revokeAccountSponsorship = ops.revokeAccountSponsorship;
Operation.revokeTrustlineSponsorship = ops.revokeTrustlineSponsorship;
Operation.revokeOfferSponsorship = ops.revokeOfferSponsorship;
Operation.revokeDataSponsorship = ops.revokeDataSponsorship;
Operation.revokeClaimableBalanceSponsorship = ops.revokeClaimableBalanceSponsorship;
Operation.revokeLiquidityPoolSponsorship = ops.revokeLiquidityPoolSponsorship;
Operation.revokeSignerSponsorship = ops.revokeSignerSponsorship;
Operation.clawback = ops.clawback;
Operation.setTrustLineFlags = ops.setTrustLineFlags;
Operation.liquidityPoolDeposit = ops.liquidityPoolDeposit;
Operation.liquidityPoolWithdraw = ops.liquidityPoolWithdraw;
Operation.invokeHostFunction = ops.invokeHostFunction;
Operation.extendFootprintTtl = ops.extendFootprintTtl;
Operation.restoreFootprint = ops.restoreFootprint;

// these are not `xdr.Operation`s directly, but are proxies for complex but
// common versions of `Operation.invokeHostFunction`
Operation.createStellarAssetContract = ops.createStellarAssetContract;
Operation.invokeContractFunction = ops.invokeContractFunction;
Operation.createCustomContract = ops.createCustomContract;
Operation.uploadContractWasm = ops.uploadContractWasm;