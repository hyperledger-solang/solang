"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.changeTrust = changeTrust;
var _jsXdr = require("@stellar/js-xdr");
var _bignumber = _interopRequireDefault(require("../util/bignumber"));
var _xdr = _interopRequireDefault(require("../xdr"));
var _asset = require("../asset");
var _liquidity_pool_asset = require("../liquidity_pool_asset");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
var MAX_INT64 = '9223372036854775807';

/**
 * Returns an XDR ChangeTrustOp. A "change trust" operation adds, removes, or updates a
 * trust line for a given asset from the source account to another.
 * @function
 * @alias Operation.changeTrust
 * @param {object} opts Options object
 * @param {Asset | LiquidityPoolAsset} opts.asset - The asset for the trust line.
 * @param {string} [opts.limit] - The limit for the asset, defaults to max int64.
 *                                If the limit is set to "0" it deletes the trustline.
 * @param {string} [opts.source] - The source account (defaults to transaction source).
 * @returns {xdr.ChangeTrustOp} Change Trust operation
 */
function changeTrust(opts) {
  var attributes = {};
  if (opts.asset instanceof _asset.Asset) {
    attributes.line = opts.asset.toChangeTrustXDRObject();
  } else if (opts.asset instanceof _liquidity_pool_asset.LiquidityPoolAsset) {
    attributes.line = opts.asset.toXDRObject();
  } else {
    throw new TypeError('asset must be Asset or LiquidityPoolAsset');
  }
  if (opts.limit !== undefined && !this.isValidAmount(opts.limit, true)) {
    throw new TypeError(this.constructAmountRequirementsError('limit'));
  }
  if (opts.limit) {
    attributes.limit = this._toXDRAmount(opts.limit);
  } else {
    attributes.limit = _jsXdr.Hyper.fromString(new _bignumber["default"](MAX_INT64).toString());
  }
  if (opts.source) {
    attributes.source = opts.source.masterKeypair;
  }
  var changeTrustOP = new _xdr["default"].ChangeTrustOp(attributes);
  var opAttributes = {};
  opAttributes.body = _xdr["default"].OperationBody.changeTrust(changeTrustOP);
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}