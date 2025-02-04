"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.liquidityPoolWithdraw = liquidityPoolWithdraw;
var _xdr = _interopRequireDefault(require("../xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Creates a liquidity pool withdraw operation.
 *
 * @function
 * @alias Operation.liquidityPoolWithdraw
 * @see https://developers.stellar.org/docs/start/list-of-operations/#liquidity-pool-withdraw
 *
 * @param {object} opts - Options object
 * @param {string} opts.liquidityPoolId - The liquidity pool ID.
 * @param {string} opts.amount - Amount of pool shares to withdraw.
 * @param {string} opts.minAmountA - Minimum amount of first asset to withdraw.
 * @param {string} opts.minAmountB - Minimum amount of second asset to withdraw.
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 *
 * @returns {xdr.Operation}   The resulting operation (xdr.LiquidityPoolWithdrawOp).
 */
function liquidityPoolWithdraw() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  var attributes = {};
  if (!opts.liquidityPoolId) {
    throw new TypeError('liquidityPoolId argument is required');
  }
  attributes.liquidityPoolId = _xdr["default"].PoolId.fromXDR(opts.liquidityPoolId, 'hex');
  if (!this.isValidAmount(opts.amount)) {
    throw new TypeError(this.constructAmountRequirementsError('amount'));
  }
  attributes.amount = this._toXDRAmount(opts.amount);
  if (!this.isValidAmount(opts.minAmountA, true)) {
    throw new TypeError(this.constructAmountRequirementsError('minAmountA'));
  }
  attributes.minAmountA = this._toXDRAmount(opts.minAmountA);
  if (!this.isValidAmount(opts.minAmountB, true)) {
    throw new TypeError(this.constructAmountRequirementsError('minAmountB'));
  }
  attributes.minAmountB = this._toXDRAmount(opts.minAmountB);
  var liquidityPoolWithdrawOp = new _xdr["default"].LiquidityPoolWithdrawOp(attributes);
  var opAttributes = {
    body: _xdr["default"].OperationBody.liquidityPoolWithdraw(liquidityPoolWithdrawOp)
  };
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}