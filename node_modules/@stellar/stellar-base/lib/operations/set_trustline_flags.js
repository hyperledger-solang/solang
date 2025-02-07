"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.setTrustLineFlags = setTrustLineFlags;
var _xdr = _interopRequireDefault(require("../xdr"));
var _keypair = require("../keypair");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
/**
 * Creates a trustline flag configuring operation.
 *
 * For the flags, set them to true to enable them and false to disable them. Any
 * unmodified operations will be marked `undefined` in the result.
 *
 * Note that you can only **clear** the clawbackEnabled flag set; it must be set
 * account-wide via operations.SetOptions (setting
 * xdr.AccountFlags.clawbackEnabled).
 *
 * @function
 * @alias Operation.setTrustLineFlags
 *
 * @param {object} opts - Options object
 * @param {string} opts.trustor     - the account whose trustline this is
 * @param {Asset}  opts.asset       - the asset on the trustline
 * @param {object} opts.flags       - the set of flags to modify
 *
 * @param {bool}   [opts.flags.authorized]  - authorize account to perform
 *     transactions with its credit
 * @param {bool}   [opts.flags.authorizedToMaintainLiabilities] - authorize
 *     account to maintain and reduce liabilities for its credit
 * @param {bool}   [opts.flags.clawbackEnabled] - stop claimable balances on
 *     this trustlines from having clawbacks enabled (this flag can only be set
 *     to false!)
 * @param {string} [opts.source] - The source account for the operation.
 *                                 Defaults to the transaction's source account.
 *
 * @note You must include at least one flag.
 *
 * @return {xdr.SetTrustLineFlagsOp}
 *
 * @link xdr.AccountFlags
 * @link xdr.TrustLineFlags
 * @see https://github.com/stellar/stellar-protocol/blob/master/core/cap-0035.md#set-trustline-flags-operation
 * @see https://developers.stellar.org/docs/start/list-of-operations/#set-options
 */
function setTrustLineFlags() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  var attributes = {};
  if (_typeof(opts.flags) !== 'object' || Object.keys(opts.flags).length === 0) {
    throw new Error('opts.flags must be a map of boolean flags to modify');
  }
  var mapping = {
    authorized: _xdr["default"].TrustLineFlags.authorizedFlag(),
    authorizedToMaintainLiabilities: _xdr["default"].TrustLineFlags.authorizedToMaintainLiabilitiesFlag(),
    clawbackEnabled: _xdr["default"].TrustLineFlags.trustlineClawbackEnabledFlag()
  };

  /* eslint no-bitwise: "off" */
  var clearFlag = 0;
  var setFlag = 0;
  Object.keys(opts.flags).forEach(function (flagName) {
    if (!Object.prototype.hasOwnProperty.call(mapping, flagName)) {
      throw new Error("unsupported flag name specified: ".concat(flagName));
    }
    var flagValue = opts.flags[flagName];
    var bit = mapping[flagName].value;
    if (flagValue === true) {
      setFlag |= bit;
    } else if (flagValue === false) {
      clearFlag |= bit;
    }
  });
  attributes.trustor = _keypair.Keypair.fromPublicKey(opts.trustor).xdrAccountId();
  attributes.asset = opts.asset.toXDRObject();
  attributes.clearFlags = clearFlag;
  attributes.setFlags = setFlag;
  var opAttributes = {
    body: _xdr["default"].OperationBody.setTrustLineFlags(new _xdr["default"].SetTrustLineFlagsOp(attributes))
  };
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}