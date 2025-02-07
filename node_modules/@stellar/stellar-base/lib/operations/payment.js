"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.payment = payment;
var _xdr = _interopRequireDefault(require("../xdr"));
var _decode_encode_muxed_account = require("../util/decode_encode_muxed_account");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Create a payment operation.
 *
 * @function
 * @alias Operation.payment
 * @see https://developers.stellar.org/docs/start/list-of-operations/#payment
 *
 * @param {object}  opts - Options object
 * @param {string}  opts.destination  - destination account ID
 * @param {Asset}   opts.asset        - asset to send
 * @param {string}  opts.amount       - amount to send
 *
 * @param {string}  [opts.source]     - The source account for the payment.
 *     Defaults to the transaction's source account.
 *
 * @returns {xdr.Operation}   The resulting payment operation (xdr.PaymentOp)
 */
function payment(opts) {
  if (!opts.asset) {
    throw new Error('Must provide an asset for a payment operation');
  }
  if (!this.isValidAmount(opts.amount)) {
    throw new TypeError(this.constructAmountRequirementsError('amount'));
  }
  var attributes = {};
  try {
    attributes.destination = (0, _decode_encode_muxed_account.decodeAddressToMuxedAccount)(opts.destination);
  } catch (e) {
    throw new Error('destination is invalid');
  }
  attributes.asset = opts.asset.toXDRObject();
  attributes.amount = this._toXDRAmount(opts.amount);
  var paymentOp = new _xdr["default"].PaymentOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.payment(paymentOp);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}