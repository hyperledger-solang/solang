"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.createAccount = createAccount;
var _xdr = _interopRequireDefault(require("../xdr"));
var _keypair = require("../keypair");
var _strkey = require("../strkey");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
/**
 * Create and fund a non existent account.
 * @function
 * @alias Operation.createAccount
 * @param {object} opts Options object
 * @param {string} opts.destination - Destination account ID to create an account for.
 * @param {string} opts.startingBalance - Amount in XLM the account should be funded for. Must be greater
 *                                   than the [reserve balance amount](https://developers.stellar.org/docs/glossary/fees/).
 * @param {string} [opts.source] - The source account for the payment. Defaults to the transaction's source account.
 * @returns {xdr.CreateAccountOp} Create account operation
 */
function createAccount(opts) {
  if (!_strkey.StrKey.isValidEd25519PublicKey(opts.destination)) {
    throw new Error('destination is invalid');
  }
  if (!this.isValidAmount(opts.startingBalance, true)) {
    throw new TypeError(this.constructAmountRequirementsError('startingBalance'));
  }
  var attributes = {};
  attributes.destination = _keypair.Keypair.fromPublicKey(opts.destination).xdrAccountId();
  attributes.startingBalance = this._toXDRAmount(opts.startingBalance);
  var createAccountOp = new _xdr["default"].CreateAccountOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.createAccount(createAccountOp);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}