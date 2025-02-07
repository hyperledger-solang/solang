"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.pathPaymentStrictReceive = pathPaymentStrictReceive;
var _xdr = _interopRequireDefault(require("../xdr"));
var _decode_encode_muxed_account = require("../util/decode_encode_muxed_account");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Creates a PathPaymentStrictReceive operation.
 *
 * A `PathPaymentStrictReceive` operation sends the specified amount to the
 * destination account. It credits the destination with `destAmount` of
 * `destAsset`, while debiting at most `sendMax` of `sendAsset` from the source.
 * The transfer optionally occurs through a path. XLM payments create the
 * destination account if it does not exist.
 *
 * @function
 * @alias Operation.pathPaymentStrictReceive
 * @see https://developers.stellar.org/docs/start/list-of-operations/#path-payment-strict-receive
 *
 * @param {object}  opts - Options object
 * @param {Asset}   opts.sendAsset    - asset to pay with
 * @param {string}  opts.sendMax      - maximum amount of sendAsset to send
 * @param {string}  opts.destination  - destination account to send to
 * @param {Asset}   opts.destAsset    - asset the destination will receive
 * @param {string}  opts.destAmount   - amount the destination receives
 * @param {Asset[]} opts.path         - array of Asset objects to use as the path
 *
 * @param {string}  [opts.source]     - The source account for the payment.
 *     Defaults to the transaction's source account.
 *
 * @returns {xdr.PathPaymentStrictReceiveOp} the resulting path payment op
 */
function pathPaymentStrictReceive(opts) {
  switch (true) {
    case !opts.sendAsset:
      throw new Error('Must specify a send asset');
    case !this.isValidAmount(opts.sendMax):
      throw new TypeError(this.constructAmountRequirementsError('sendMax'));
    case !opts.destAsset:
      throw new Error('Must provide a destAsset for a payment operation');
    case !this.isValidAmount(opts.destAmount):
      throw new TypeError(this.constructAmountRequirementsError('destAmount'));
    default:
      break;
  }
  var attributes = {};
  attributes.sendAsset = opts.sendAsset.toXDRObject();
  attributes.sendMax = this._toXDRAmount(opts.sendMax);
  try {
    attributes.destination = (0, _decode_encode_muxed_account.decodeAddressToMuxedAccount)(opts.destination);
  } catch (e) {
    throw new Error('destination is invalid');
  }
  attributes.destAsset = opts.destAsset.toXDRObject();
  attributes.destAmount = this._toXDRAmount(opts.destAmount);
  var path = opts.path ? opts.path : [];
  attributes.path = path.map(function (x) {
    return x.toXDRObject();
  });
  var payment = new _xdr["default"].PathPaymentStrictReceiveOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.pathPaymentStrictReceive(payment);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}