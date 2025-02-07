"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.pathPaymentStrictSend = pathPaymentStrictSend;
var _xdr = _interopRequireDefault(require("../xdr"));
var _decode_encode_muxed_account = require("../util/decode_encode_muxed_account");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Creates a PathPaymentStrictSend operation.
 *
 * A `PathPaymentStrictSend` operation sends the specified amount to the
 * destination account crediting at least `destMin` of `destAsset`, optionally
 * through a path. XLM payments create the destination account if it does not
 * exist.
 *
 * @function
 * @alias Operation.pathPaymentStrictSend
 * @see https://developers.stellar.org/docs/start/list-of-operations/#path-payment-strict-send
 *
 * @param {object}  opts - Options object
 * @param {Asset}   opts.sendAsset    - asset to pay with
 * @param {string}  opts.sendAmount   - amount of sendAsset to send (excluding fees)
 * @param {string}  opts.destination  - destination account to send to
 * @param {Asset}   opts.destAsset    - asset the destination will receive
 * @param {string}  opts.destMin      - minimum amount of destAsset to be receive
 * @param {Asset[]} opts.path         - array of Asset objects to use as the path
 *
 * @param {string}  [opts.source]     - The source account for the payment.
 *     Defaults to the transaction's source account.
 *
 * @returns {xdr.Operation}   the resulting path payment operation
 *     (xdr.PathPaymentStrictSendOp)
 */
function pathPaymentStrictSend(opts) {
  switch (true) {
    case !opts.sendAsset:
      throw new Error('Must specify a send asset');
    case !this.isValidAmount(opts.sendAmount):
      throw new TypeError(this.constructAmountRequirementsError('sendAmount'));
    case !opts.destAsset:
      throw new Error('Must provide a destAsset for a payment operation');
    case !this.isValidAmount(opts.destMin):
      throw new TypeError(this.constructAmountRequirementsError('destMin'));
    default:
      break;
  }
  var attributes = {};
  attributes.sendAsset = opts.sendAsset.toXDRObject();
  attributes.sendAmount = this._toXDRAmount(opts.sendAmount);
  try {
    attributes.destination = (0, _decode_encode_muxed_account.decodeAddressToMuxedAccount)(opts.destination);
  } catch (e) {
    throw new Error('destination is invalid');
  }
  attributes.destAsset = opts.destAsset.toXDRObject();
  attributes.destMin = this._toXDRAmount(opts.destMin);
  var path = opts.path ? opts.path : [];
  attributes.path = path.map(function (x) {
    return x.toXDRObject();
  });
  var payment = new _xdr["default"].PathPaymentStrictSendOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.pathPaymentStrictSend(payment);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}