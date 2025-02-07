"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.clawback = clawback;
var _xdr = _interopRequireDefault(require("../xdr"));
var _decode_encode_muxed_account = require("../util/decode_encode_muxed_account");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Creates a clawback operation.
 *
 * @function
 * @alias Operation.clawback
 *
 * @param {object} opts - Options object
 * @param {Asset}  opts.asset   - The asset being clawed back.
 * @param {string} opts.amount  - The amount of the asset to claw back.
 * @param {string} opts.from    - The public key of the (optionally-muxed)
 *     account to claw back from.
 *
 * @param {string} [opts.source] - The source account for the operation.
 *     Defaults to the transaction's source account.
 *
 * @return {xdr.ClawbackOp}
 *
 * @see https://github.com/stellar/stellar-protocol/blob/master/core/cap-0035.md#clawback-operation
 */
function clawback(opts) {
  var attributes = {};
  if (!this.isValidAmount(opts.amount)) {
    throw new TypeError(this.constructAmountRequirementsError('amount'));
  }
  attributes.amount = this._toXDRAmount(opts.amount);
  attributes.asset = opts.asset.toXDRObject();
  try {
    attributes.from = (0, _decode_encode_muxed_account.decodeAddressToMuxedAccount)(opts.from);
  } catch (e) {
    throw new Error('from address is invalid');
  }
  var opAttributes = {
    body: _xdr["default"].OperationBody.clawback(new _xdr["default"].ClawbackOp(attributes))
  };
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}