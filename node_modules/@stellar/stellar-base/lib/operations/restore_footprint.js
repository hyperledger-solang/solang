"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.restoreFootprint = restoreFootprint;
var _xdr = _interopRequireDefault(require("../xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Builds an operation to restore the archived ledger entries specified
 * by the ledger keys.
 *
 * The ledger keys to restore are specified separately from the operation
 * in read-write footprint of the transaction.
 *
 * It takes no parameters because the relevant footprint is derived from the
 * transaction itself. See {@link TransactionBuilder}'s `opts.sorobanData`
 * parameter (or {@link TransactionBuilder.setSorobanData} /
 * {@link TransactionBuilder.setLedgerKeys}), which is a
 * {@link xdr.SorobanTransactionData} instance that contains fee data & resource
 * usage as part of {@link xdr.SorobanTransactionData}.
 *
 * @function
 * @alias Operation.restoreFootprint
 *
 * @param {object} [opts] - an optional set of parameters
 * @param {string} [opts.source] - an optional source account
 *
 * @returns {xdr.Operation} a Bump Footprint Expiration operation
 *    (xdr.RestoreFootprintOp)
 */
function restoreFootprint() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  var op = new _xdr["default"].RestoreFootprintOp({
    ext: new _xdr["default"].ExtensionPoint(0)
  });
  var opAttributes = {
    body: _xdr["default"].OperationBody.restoreFootprint(op)
  };
  this.setSourceAccount(opAttributes, opts !== null && opts !== void 0 ? opts : {});
  return new _xdr["default"].Operation(opAttributes);
}