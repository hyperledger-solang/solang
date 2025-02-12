"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.claimClaimableBalance = claimClaimableBalance;
exports.validateClaimableBalanceId = validateClaimableBalanceId;
var _xdr = _interopRequireDefault(require("../xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Create a new claim claimable balance operation.
 * @function
 * @alias Operation.claimClaimableBalance
 * @param {object} opts Options object
 * @param {string} opts.balanceId - The claimable balance id to be claimed.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} Claim claimable balance operation
 *
 * @example
 * const op = Operation.claimClaimableBalance({
 *   balanceId: '00000000da0d57da7d4850e7fc10d2a9d0ebc731f7afb40574c03395b17d49149b91f5be',
 * });
 *
 */
function claimClaimableBalance() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  validateClaimableBalanceId(opts.balanceId);
  var attributes = {};
  attributes.balanceId = _xdr["default"].ClaimableBalanceId.fromXDR(opts.balanceId, 'hex');
  var claimClaimableBalanceOp = new _xdr["default"].ClaimClaimableBalanceOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.claimClaimableBalance(claimClaimableBalanceOp);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}
function validateClaimableBalanceId(balanceId) {
  if (typeof balanceId !== 'string' || balanceId.length !== 8 + 64 /* 8b discriminant + 64b string */) {
    throw new Error('must provide a valid claimable balance id');
  }
}