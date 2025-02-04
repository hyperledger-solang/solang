"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.setOptions = setOptions;
var _xdr = _interopRequireDefault(require("../xdr"));
var _keypair = require("../keypair");
var _strkey = require("../strkey");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/* eslint-disable no-param-reassign */

function weightCheckFunction(value, name) {
  if (value >= 0 && value <= 255) {
    return true;
  }
  throw new Error("".concat(name, " value must be between 0 and 255"));
}

/**
 * Returns an XDR SetOptionsOp. A "set options" operations set or clear account flags,
 * set the account's inflation destination, and/or add new signers to the account.
 * The flags used in `opts.clearFlags` and `opts.setFlags` can be the following:
 *   - `{@link AuthRequiredFlag}`
 *   - `{@link AuthRevocableFlag}`
 *   - `{@link AuthImmutableFlag}`
 *   - `{@link AuthClawbackEnabledFlag}`
 *
 * It's possible to set/clear multiple flags at once using logical or.
 *
 * @function
 * @alias Operation.setOptions
 *
 * @param {object} opts Options object
 * @param {string} [opts.inflationDest] - Set this account ID as the account's inflation destination.
 * @param {(number|string)} [opts.clearFlags] - Bitmap integer for which account flags to clear.
 * @param {(number|string)} [opts.setFlags] - Bitmap integer for which account flags to set.
 * @param {number|string} [opts.masterWeight] - The master key weight.
 * @param {number|string} [opts.lowThreshold] - The sum weight for the low threshold.
 * @param {number|string} [opts.medThreshold] - The sum weight for the medium threshold.
 * @param {number|string} [opts.highThreshold] - The sum weight for the high threshold.
 * @param {object} [opts.signer] - Add or remove a signer from the account. The signer is
 *                                 deleted if the weight is 0. Only one of `ed25519PublicKey`, `sha256Hash`, `preAuthTx` should be defined.
 * @param {string} [opts.signer.ed25519PublicKey] - The ed25519 public key of the signer.
 * @param {Buffer|string} [opts.signer.sha256Hash] - sha256 hash (Buffer or hex string) of preimage that will unlock funds. Preimage should be used as signature of future transaction.
 * @param {Buffer|string} [opts.signer.preAuthTx] - Hash (Buffer or hex string) of transaction that will unlock funds.
 * @param {string} [opts.signer.ed25519SignedPayload] - Signed payload signer (ed25519 public key + raw payload) for atomic transaction signature disclosure.
 * @param {number|string} [opts.signer.weight] - The weight of the new signer (0 to delete or 1-255)
 * @param {string} [opts.homeDomain] - sets the home domain used for reverse federation lookup.
 * @param {string} [opts.source] - The source account (defaults to transaction source).
 *
 * @returns {xdr.SetOptionsOp}  XDR operation
 * @see [Account flags](https://developers.stellar.org/docs/glossary/accounts/#flags)
 */
function setOptions(opts) {
  var attributes = {};
  if (opts.inflationDest) {
    if (!_strkey.StrKey.isValidEd25519PublicKey(opts.inflationDest)) {
      throw new Error('inflationDest is invalid');
    }
    attributes.inflationDest = _keypair.Keypair.fromPublicKey(opts.inflationDest).xdrAccountId();
  }
  attributes.clearFlags = this._checkUnsignedIntValue('clearFlags', opts.clearFlags);
  attributes.setFlags = this._checkUnsignedIntValue('setFlags', opts.setFlags);
  attributes.masterWeight = this._checkUnsignedIntValue('masterWeight', opts.masterWeight, weightCheckFunction);
  attributes.lowThreshold = this._checkUnsignedIntValue('lowThreshold', opts.lowThreshold, weightCheckFunction);
  attributes.medThreshold = this._checkUnsignedIntValue('medThreshold', opts.medThreshold, weightCheckFunction);
  attributes.highThreshold = this._checkUnsignedIntValue('highThreshold', opts.highThreshold, weightCheckFunction);
  if (opts.homeDomain !== undefined && typeof opts.homeDomain !== 'string') {
    throw new TypeError('homeDomain argument must be of type String');
  }
  attributes.homeDomain = opts.homeDomain;
  if (opts.signer) {
    var weight = this._checkUnsignedIntValue('signer.weight', opts.signer.weight, weightCheckFunction);
    var key;
    var setValues = 0;
    if (opts.signer.ed25519PublicKey) {
      if (!_strkey.StrKey.isValidEd25519PublicKey(opts.signer.ed25519PublicKey)) {
        throw new Error('signer.ed25519PublicKey is invalid.');
      }
      var rawKey = _strkey.StrKey.decodeEd25519PublicKey(opts.signer.ed25519PublicKey);

      // eslint-disable-next-line new-cap
      key = new _xdr["default"].SignerKey.signerKeyTypeEd25519(rawKey);
      setValues += 1;
    }
    if (opts.signer.preAuthTx) {
      if (typeof opts.signer.preAuthTx === 'string') {
        opts.signer.preAuthTx = Buffer.from(opts.signer.preAuthTx, 'hex');
      }
      if (!(Buffer.isBuffer(opts.signer.preAuthTx) && opts.signer.preAuthTx.length === 32)) {
        throw new Error('signer.preAuthTx must be 32 bytes Buffer.');
      }

      // eslint-disable-next-line new-cap
      key = new _xdr["default"].SignerKey.signerKeyTypePreAuthTx(opts.signer.preAuthTx);
      setValues += 1;
    }
    if (opts.signer.sha256Hash) {
      if (typeof opts.signer.sha256Hash === 'string') {
        opts.signer.sha256Hash = Buffer.from(opts.signer.sha256Hash, 'hex');
      }
      if (!(Buffer.isBuffer(opts.signer.sha256Hash) && opts.signer.sha256Hash.length === 32)) {
        throw new Error('signer.sha256Hash must be 32 bytes Buffer.');
      }

      // eslint-disable-next-line new-cap
      key = new _xdr["default"].SignerKey.signerKeyTypeHashX(opts.signer.sha256Hash);
      setValues += 1;
    }
    if (opts.signer.ed25519SignedPayload) {
      if (!_strkey.StrKey.isValidSignedPayload(opts.signer.ed25519SignedPayload)) {
        throw new Error('signer.ed25519SignedPayload is invalid.');
      }
      var _rawKey = _strkey.StrKey.decodeSignedPayload(opts.signer.ed25519SignedPayload);
      var signedPayloadXdr = _xdr["default"].SignerKeyEd25519SignedPayload.fromXDR(_rawKey);

      // eslint-disable-next-line new-cap
      key = _xdr["default"].SignerKey.signerKeyTypeEd25519SignedPayload(signedPayloadXdr);
      setValues += 1;
    }
    if (setValues !== 1) {
      throw new Error('Signer object must contain exactly one of signer.ed25519PublicKey, signer.sha256Hash, signer.preAuthTx.');
    }
    attributes.signer = new _xdr["default"].Signer({
      key: key,
      weight: weight
    });
  }
  var setOptionsOp = new _xdr["default"].SetOptionsOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.setOptions(setOptionsOp);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}