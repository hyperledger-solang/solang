"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.accountMerge = accountMerge;
var _xdr = _interopRequireDefault(require("../xdr"));
var _decode_encode_muxed_account = require("../util/decode_encode_muxed_account");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Transfers native balance to destination account.
 *
 * @function
 * @alias Operation.accountMerge
 *
 * @param {object} opts - options object
 * @param {string} opts.destination - destination to merge the source account into
 * @param {string} [opts.source]    - operation source account (defaults to
 *     transaction source)
 *
 * @returns {xdr.Operation} an Account Merge operation (xdr.AccountMergeOp)
 */
function accountMerge(opts) {
  var opAttributes = {};
  try {
    opAttributes.body = _xdr["default"].OperationBody.accountMerge((0, _decode_encode_muxed_account.decodeAddressToMuxedAccount)(opts.destination));
  } catch (e) {
    throw new Error('destination is invalid');
  }
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}