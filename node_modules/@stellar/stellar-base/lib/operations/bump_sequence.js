"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bumpSequence = bumpSequence;
var _jsXdr = require("@stellar/js-xdr");
var _bignumber = _interopRequireDefault(require("../util/bignumber"));
var _xdr = _interopRequireDefault(require("../xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * This operation bumps sequence number.
 * @function
 * @alias Operation.bumpSequence
 * @param {object} opts Options object
 * @param {string} opts.bumpTo - Sequence number to bump to.
 * @param {string} [opts.source] - The optional source account.
 * @returns {xdr.BumpSequenceOp} Operation
 */
function bumpSequence(opts) {
  var attributes = {};
  if (typeof opts.bumpTo !== 'string') {
    throw new Error('bumpTo must be a string');
  }
  try {
    // eslint-disable-next-line no-new
    new _bignumber["default"](opts.bumpTo);
  } catch (e) {
    throw new Error('bumpTo must be a stringified number');
  }
  attributes.bumpTo = _jsXdr.Hyper.fromString(opts.bumpTo);
  var bumpSequenceOp = new _xdr["default"].BumpSequenceOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.bumpSequence(bumpSequenceOp);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}