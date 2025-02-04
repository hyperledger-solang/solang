"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.clawbackClaimableBalance = clawbackClaimableBalance;
var _xdr = _interopRequireDefault(require("../xdr"));
var _claim_claimable_balance = require("./claim_claimable_balance");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Creates a clawback operation for a claimable balance.
 *
 * @function
 * @alias Operation.clawbackClaimableBalance
 * @param {object} opts - Options object
 * @param {string} opts.balanceId - The claimable balance ID to be clawed back.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 *
 * @return {xdr.ClawbackClaimableBalanceOp}
 *
 * @example
 * const op = Operation.clawbackClaimableBalance({
 *   balanceId: '00000000da0d57da7d4850e7fc10d2a9d0ebc731f7afb40574c03395b17d49149b91f5be',
 * });
 *
 * @link https://github.com/stellar/stellar-protocol/blob/master/core/cap-0035.md#clawback-claimable-balance-operation
 */
function clawbackClaimableBalance() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  (0, _claim_claimable_balance.validateClaimableBalanceId)(opts.balanceId);
  var attributes = {
    balanceId: _xdr["default"].ClaimableBalanceId.fromXDR(opts.balanceId, 'hex')
  };
  var opAttributes = {
    body: _xdr["default"].OperationBody.clawbackClaimableBalance(new _xdr["default"].ClawbackClaimableBalanceOp(attributes))
  };
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}