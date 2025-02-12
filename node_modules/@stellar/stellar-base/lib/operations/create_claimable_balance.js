"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.createClaimableBalance = createClaimableBalance;
var _xdr = _interopRequireDefault(require("../xdr"));
var _asset = require("../asset");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Create a new claimable balance operation.
 *
 * @function
 * @alias Operation.createClaimableBalance
 *
 * @param {object} opts Options object
 * @param {Asset} opts.asset - The asset for the claimable balance.
 * @param {string} opts.amount - Amount.
 * @param {Claimant[]} opts.claimants - An array of Claimants
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 *
 * @returns {xdr.Operation} Create claimable balance operation
 *
 * @example
 * const asset = new Asset(
 *   'USD',
 *   'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7'
 * );
 * const amount = '100.0000000';
 * const claimants = [
 *   new Claimant(
 *     'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGSNFHEYVXM3XOJMDS674JZ',
 *      Claimant.predicateBeforeAbsoluteTime("4102444800000")
 *   )
 * ];
 *
 * const op = Operation.createClaimableBalance({
 *   asset,
 *   amount,
 *   claimants
 * });
 *
 */
function createClaimableBalance(opts) {
  if (!(opts.asset instanceof _asset.Asset)) {
    throw new Error('must provide an asset for create claimable balance operation');
  }
  if (!this.isValidAmount(opts.amount)) {
    throw new TypeError(this.constructAmountRequirementsError('amount'));
  }
  if (!Array.isArray(opts.claimants) || opts.claimants.length === 0) {
    throw new Error('must provide at least one claimant');
  }
  var attributes = {};
  attributes.asset = opts.asset.toXDRObject();
  attributes.amount = this._toXDRAmount(opts.amount);
  attributes.claimants = Object.values(opts.claimants).map(function (c) {
    return c.toXDRObject();
  });
  var createClaimableBalanceOp = new _xdr["default"].CreateClaimableBalanceOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.createClaimableBalance(createClaimableBalanceOp);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}