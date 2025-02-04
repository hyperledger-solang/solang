"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.allowTrust = allowTrust;
var _xdr = _interopRequireDefault(require("../xdr"));
var _keypair = require("../keypair");
var _strkey = require("../strkey");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * @deprecated since v5.0
 *
 * Returns an XDR AllowTrustOp. An "allow trust" operation authorizes another
 * account to hold your account's credit for a given asset.
 *
 * @function
 * @alias Operation.allowTrust
 *
 * @param {object} opts Options object
 * @param {string} opts.trustor - The trusting account (the one being authorized)
 * @param {string} opts.assetCode - The asset code being authorized.
 * @param {(0|1|2)} opts.authorize - `1` to authorize, `2` to authorize to maintain liabilities, and `0` to deauthorize.
 * @param {string} [opts.source] - The source account (defaults to transaction source).
 *
 * @returns {xdr.AllowTrustOp} Allow Trust operation
 */
function allowTrust(opts) {
  if (!_strkey.StrKey.isValidEd25519PublicKey(opts.trustor)) {
    throw new Error('trustor is invalid');
  }
  var attributes = {};
  attributes.trustor = _keypair.Keypair.fromPublicKey(opts.trustor).xdrAccountId();
  if (opts.assetCode.length <= 4) {
    var code = opts.assetCode.padEnd(4, '\0');
    attributes.asset = _xdr["default"].AssetCode.assetTypeCreditAlphanum4(code);
  } else if (opts.assetCode.length <= 12) {
    var _code = opts.assetCode.padEnd(12, '\0');
    attributes.asset = _xdr["default"].AssetCode.assetTypeCreditAlphanum12(_code);
  } else {
    throw new Error('Asset code must be 12 characters at max.');
  }
  if (typeof opts.authorize === 'boolean') {
    if (opts.authorize) {
      attributes.authorize = _xdr["default"].TrustLineFlags.authorizedFlag().value;
    } else {
      attributes.authorize = 0;
    }
  } else {
    attributes.authorize = opts.authorize;
  }
  var allowTrustOp = new _xdr["default"].AllowTrustOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.allowTrust(allowTrustOp);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}