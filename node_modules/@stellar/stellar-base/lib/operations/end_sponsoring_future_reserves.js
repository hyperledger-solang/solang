"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.endSponsoringFutureReserves = endSponsoringFutureReserves;
var _xdr = _interopRequireDefault(require("../xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Create an "end sponsoring future reserves" operation.
 * @function
 * @alias Operation.endSponsoringFutureReserves
 * @param {object} opts Options object
 * @param {string} [opts.source] - The source account for the operation. Defaults to the transaction's source account.
 * @returns {xdr.Operation} xdr operation
 *
 * @example
 * const op = Operation.endSponsoringFutureReserves();
 *
 */
function endSponsoringFutureReserves() {
  var opts = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.endSponsoringFutureReserves();
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}