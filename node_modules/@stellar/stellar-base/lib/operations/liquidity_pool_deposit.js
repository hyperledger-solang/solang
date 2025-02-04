"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.liquidityPoolDeposit = liquidityPoolDeposit;
var _xdr = _interopRequireDefault(require("../xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Creates a liquidity pool deposit operation.
 *
 * @function
 * @alias Operation.liquidityPoolDeposit
 * @see https://developers.stellar.org/docs/start/list-of-operations/#liquidity-pool-deposit
 *
 * @param {object} opts - Options object
 * @param {string} opts.liquidityPoolId - The liquidity pool ID.
 * @param {string} opts.maxAmountA - Maximum amount of first asset to deposit.
 * @param {string} opts.maxAmountB - Maximum amount of second asset to deposit.
 * @param {number|string|BigNumber|Object} opts.minPrice -  Minimum depositA/depositB price.
 * @param {number} opts.minPrice.n - If `opts.minPrice` is an object: the price numerator
 * @param {number} opts.minPrice.d - If `opts.minPrice` is an object: the price denominator
 * @param {number|string|BigNumber|Object} opts.maxPrice -  Maximum depositA/depositB price.
 * @param {number} opts.maxPrice.n - If `opts.maxPrice` is an object: the price numerator
 * @param {number} opts.maxPrice.d - If `opts.maxPrice` is an object: the price denominator
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 *
 * @returns {xdr.Operation} The resulting operation (xdr.LiquidityPoolDepositOp).
 */
function liquidityPoolDeposit() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  var liquidityPoolId = opts.liquidityPoolId,
    maxAmountA = opts.maxAmountA,
    maxAmountB = opts.maxAmountB,
    minPrice = opts.minPrice,
    maxPrice = opts.maxPrice;
  var attributes = {};
  if (!liquidityPoolId) {
    throw new TypeError('liquidityPoolId argument is required');
  }
  attributes.liquidityPoolId = _xdr["default"].PoolId.fromXDR(liquidityPoolId, 'hex');
  if (!this.isValidAmount(maxAmountA, true)) {
    throw new TypeError(this.constructAmountRequirementsError('maxAmountA'));
  }
  attributes.maxAmountA = this._toXDRAmount(maxAmountA);
  if (!this.isValidAmount(maxAmountB, true)) {
    throw new TypeError(this.constructAmountRequirementsError('maxAmountB'));
  }
  attributes.maxAmountB = this._toXDRAmount(maxAmountB);
  if (minPrice === undefined) {
    throw new TypeError('minPrice argument is required');
  }
  attributes.minPrice = this._toXDRPrice(minPrice);
  if (maxPrice === undefined) {
    throw new TypeError('maxPrice argument is required');
  }
  attributes.maxPrice = this._toXDRPrice(maxPrice);
  var liquidityPoolDepositOp = new _xdr["default"].LiquidityPoolDepositOp(attributes);
  var opAttributes = {
    body: _xdr["default"].OperationBody.liquidityPoolDeposit(liquidityPoolDepositOp)
  };
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}