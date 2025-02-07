"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.revokeAccountSponsorship = revokeAccountSponsorship;
exports.revokeClaimableBalanceSponsorship = revokeClaimableBalanceSponsorship;
exports.revokeDataSponsorship = revokeDataSponsorship;
exports.revokeLiquidityPoolSponsorship = revokeLiquidityPoolSponsorship;
exports.revokeOfferSponsorship = revokeOfferSponsorship;
exports.revokeSignerSponsorship = revokeSignerSponsorship;
exports.revokeTrustlineSponsorship = revokeTrustlineSponsorship;
var _xdr = _interopRequireDefault(require("../xdr"));
var _strkey = require("../strkey");
var _keypair = require("../keypair");
var _asset = require("../asset");
var _liquidity_pool_id = require("../liquidity_pool_id");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Create a "revoke sponsorship" operation for an account.
 *
 * @function
 * @alias Operation.revokeAccountSponsorship
 * @param {object} opts Options object
 * @param {string} opts.account - The sponsored account ID.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} xdr operation
 *
 * @example
 * const op = Operation.revokeAccountSponsorship({
 *   account: 'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7
 * });
 *
 */
function revokeAccountSponsorship() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  if (!_strkey.StrKey.isValidEd25519PublicKey(opts.account)) {
    throw new Error('account is invalid');
  }
  var ledgerKey = _xdr["default"].LedgerKey.account(new _xdr["default"].LedgerKeyAccount({
    accountId: _keypair.Keypair.fromPublicKey(opts.account).xdrAccountId()
  }));
  var op = _xdr["default"].RevokeSponsorshipOp.revokeSponsorshipLedgerEntry(ledgerKey);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.revokeSponsorship(op);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}

/**
 * Create a "revoke sponsorship" operation for a trustline.
 *
 * @function
 * @alias Operation.revokeTrustlineSponsorship
 * @param {object} opts Options object
 * @param {string} opts.account - The account ID which owns the trustline.
 * @param {Asset | LiquidityPoolId} opts.asset - The trustline asset.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} xdr operation
 *
 * @example
 * const op = Operation.revokeTrustlineSponsorship({
 *   account: 'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7
 *   asset: new StellarBase.LiquidityPoolId(
 *     'USDUSD',
 *     'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7'
 *   )
 * });
 *
 */
function revokeTrustlineSponsorship() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  if (!_strkey.StrKey.isValidEd25519PublicKey(opts.account)) {
    throw new Error('account is invalid');
  }
  var asset;
  if (opts.asset instanceof _asset.Asset) {
    asset = opts.asset.toTrustLineXDRObject();
  } else if (opts.asset instanceof _liquidity_pool_id.LiquidityPoolId) {
    asset = opts.asset.toXDRObject();
  } else {
    throw new TypeError('asset must be an Asset or LiquidityPoolId');
  }
  var ledgerKey = _xdr["default"].LedgerKey.trustline(new _xdr["default"].LedgerKeyTrustLine({
    accountId: _keypair.Keypair.fromPublicKey(opts.account).xdrAccountId(),
    asset: asset
  }));
  var op = _xdr["default"].RevokeSponsorshipOp.revokeSponsorshipLedgerEntry(ledgerKey);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.revokeSponsorship(op);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}

/**
 * Create a "revoke sponsorship" operation for an offer.
 *
 * @function
 * @alias Operation.revokeOfferSponsorship
 * @param {object} opts Options object
 * @param {string} opts.seller - The account ID which created the offer.
 * @param {string} opts.offerId - The offer ID.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} xdr operation
 *
 * @example
 * const op = Operation.revokeOfferSponsorship({
 *   seller: 'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7
 *   offerId: '1234'
 * });
 *
 */
function revokeOfferSponsorship() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  if (!_strkey.StrKey.isValidEd25519PublicKey(opts.seller)) {
    throw new Error('seller is invalid');
  }
  if (typeof opts.offerId !== 'string') {
    throw new Error('offerId is invalid');
  }
  var ledgerKey = _xdr["default"].LedgerKey.offer(new _xdr["default"].LedgerKeyOffer({
    sellerId: _keypair.Keypair.fromPublicKey(opts.seller).xdrAccountId(),
    offerId: _xdr["default"].Int64.fromString(opts.offerId)
  }));
  var op = _xdr["default"].RevokeSponsorshipOp.revokeSponsorshipLedgerEntry(ledgerKey);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.revokeSponsorship(op);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}

/**
 * Create a "revoke sponsorship" operation for a data entry.
 *
 * @function
 * @alias Operation.revokeDataSponsorship
 * @param {object} opts Options object
 * @param {string} opts.account - The account ID which owns the data entry.
 * @param {string} opts.name - The name of the data entry
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} xdr operation
 *
 * @example
 * const op = Operation.revokeDataSponsorship({
 *   account: 'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7
 *   name: 'foo'
 * });
 *
 */
function revokeDataSponsorship() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  if (!_strkey.StrKey.isValidEd25519PublicKey(opts.account)) {
    throw new Error('account is invalid');
  }
  if (typeof opts.name !== 'string' || opts.name.length > 64) {
    throw new Error('name must be a string, up to 64 characters');
  }
  var ledgerKey = _xdr["default"].LedgerKey.data(new _xdr["default"].LedgerKeyData({
    accountId: _keypair.Keypair.fromPublicKey(opts.account).xdrAccountId(),
    dataName: opts.name
  }));
  var op = _xdr["default"].RevokeSponsorshipOp.revokeSponsorshipLedgerEntry(ledgerKey);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.revokeSponsorship(op);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}

/**
 * Create a "revoke sponsorship" operation for a claimable balance.
 *
 * @function
 * @alias Operation.revokeClaimableBalanceSponsorship
 * @param {object} opts Options object
 * @param {string} opts.balanceId - The sponsored claimable balance ID.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} xdr operation
 *
 * @example
 * const op = Operation.revokeClaimableBalanceSponsorship({
 *   balanceId: '00000000da0d57da7d4850e7fc10d2a9d0ebc731f7afb40574c03395b17d49149b91f5be',
 * });
 *
 */
function revokeClaimableBalanceSponsorship() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  if (typeof opts.balanceId !== 'string') {
    throw new Error('balanceId is invalid');
  }
  var ledgerKey = _xdr["default"].LedgerKey.claimableBalance(new _xdr["default"].LedgerKeyClaimableBalance({
    balanceId: _xdr["default"].ClaimableBalanceId.fromXDR(opts.balanceId, 'hex')
  }));
  var op = _xdr["default"].RevokeSponsorshipOp.revokeSponsorshipLedgerEntry(ledgerKey);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.revokeSponsorship(op);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}

/**
 * Creates a "revoke sponsorship" operation for a liquidity pool.
 *
 * @function
 * @alias Operation.revokeLiquidityPoolSponsorship
 * @param {object} opts – Options object.
 * @param {string} opts.liquidityPoolId - The sponsored liquidity pool ID in 'hex' string.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} xdr Operation.
 *
 * @example
 * const op = Operation.revokeLiquidityPoolSponsorship({
 *   liquidityPoolId: 'dd7b1ab831c273310ddbec6f97870aa83c2fbd78ce22aded37ecbf4f3380fac7',
 * });
 *
 */
function revokeLiquidityPoolSponsorship() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  if (typeof opts.liquidityPoolId !== 'string') {
    throw new Error('liquidityPoolId is invalid');
  }
  var ledgerKey = _xdr["default"].LedgerKey.liquidityPool(new _xdr["default"].LedgerKeyLiquidityPool({
    liquidityPoolId: _xdr["default"].PoolId.fromXDR(opts.liquidityPoolId, 'hex')
  }));
  var op = _xdr["default"].RevokeSponsorshipOp.revokeSponsorshipLedgerEntry(ledgerKey);
  var opAttributes = {
    body: _xdr["default"].OperationBody.revokeSponsorship(op)
  };
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}

/**
 * Create a "revoke sponsorship" operation for a signer.
 *
 * @function
 * @alias Operation.revokeSignerSponsorship
 * @param {object} opts Options object
 * @param {string} opts.account - The account ID where the signer sponsorship is being removed from.
 * @param {object} opts.signer - The signer whose sponsorship is being removed.
 * @param {string} [opts.signer.ed25519PublicKey] - The ed25519 public key of the signer.
 * @param {Buffer|string} [opts.signer.sha256Hash] - sha256 hash (Buffer or hex string).
 * @param {Buffer|string} [opts.signer.preAuthTx] - Hash (Buffer or hex string) of transaction.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} xdr operation
 *
 * @example
 * const op = Operation.revokeSignerSponsorship({
 *   account: 'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7
 *   signer: {
 *     ed25519PublicKey: 'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGSNFHEYVXM3XOJMDS674JZ'
 *   }
 * })
 *
 */
function revokeSignerSponsorship() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  if (!_strkey.StrKey.isValidEd25519PublicKey(opts.account)) {
    throw new Error('account is invalid');
  }
  var key;
  if (opts.signer.ed25519PublicKey) {
    if (!_strkey.StrKey.isValidEd25519PublicKey(opts.signer.ed25519PublicKey)) {
      throw new Error('signer.ed25519PublicKey is invalid.');
    }
    var rawKey = _strkey.StrKey.decodeEd25519PublicKey(opts.signer.ed25519PublicKey);
    key = new _xdr["default"].SignerKey.signerKeyTypeEd25519(rawKey);
  } else if (opts.signer.preAuthTx) {
    var buffer;
    if (typeof opts.signer.preAuthTx === 'string') {
      buffer = Buffer.from(opts.signer.preAuthTx, 'hex');
    } else {
      buffer = opts.signer.preAuthTx;
    }
    if (!(Buffer.isBuffer(buffer) && buffer.length === 32)) {
      throw new Error('signer.preAuthTx must be 32 bytes Buffer.');
    }
    key = new _xdr["default"].SignerKey.signerKeyTypePreAuthTx(buffer);
  } else if (opts.signer.sha256Hash) {
    var _buffer;
    if (typeof opts.signer.sha256Hash === 'string') {
      _buffer = Buffer.from(opts.signer.sha256Hash, 'hex');
    } else {
      _buffer = opts.signer.sha256Hash;
    }
    if (!(Buffer.isBuffer(_buffer) && _buffer.length === 32)) {
      throw new Error('signer.sha256Hash must be 32 bytes Buffer.');
    }
    key = new _xdr["default"].SignerKey.signerKeyTypeHashX(_buffer);
  } else {
    throw new Error('signer is invalid');
  }
  var signer = new _xdr["default"].RevokeSponsorshipOpSigner({
    accountId: _keypair.Keypair.fromPublicKey(opts.account).xdrAccountId(),
    signerKey: key
  });
  var op = _xdr["default"].RevokeSponsorshipOp.revokeSponsorshipSigner(signer);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.revokeSponsorship(op);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}