"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.LiquidityPoolAsset = void 0;
var _xdr = _interopRequireDefault(require("./xdr"));
var _asset = require("./asset");
var _get_liquidity_pool_id = require("./get_liquidity_pool_id");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * LiquidityPoolAsset class represents a liquidity pool trustline change.
 *
 * @constructor
 * @param {Asset} assetA – The first asset in the Pool, it must respect the rule assetA < assetB. See {@link Asset.compare} for more details on how assets are sorted.
 * @param {Asset} assetB – The second asset in the Pool, it must respect the rule assetA < assetB. See {@link Asset.compare} for more details on how assets are sorted.
 * @param {number} fee – The liquidity pool fee. For now the only fee supported is `30`.
 */
var LiquidityPoolAsset = exports.LiquidityPoolAsset = /*#__PURE__*/function () {
  function LiquidityPoolAsset(assetA, assetB, fee) {
    _classCallCheck(this, LiquidityPoolAsset);
    if (!assetA || !(assetA instanceof _asset.Asset)) {
      throw new Error('assetA is invalid');
    }
    if (!assetB || !(assetB instanceof _asset.Asset)) {
      throw new Error('assetB is invalid');
    }
    if (_asset.Asset.compare(assetA, assetB) !== -1) {
      throw new Error('Assets are not in lexicographic order');
    }
    if (!fee || fee !== _get_liquidity_pool_id.LiquidityPoolFeeV18) {
      throw new Error('fee is invalid');
    }
    this.assetA = assetA;
    this.assetB = assetB;
    this.fee = fee;
  }

  /**
   * Returns a liquidity pool asset object from its XDR ChangeTrustAsset object
   * representation.
   * @param {xdr.ChangeTrustAsset} ctAssetXdr - The asset XDR object.
   * @returns {LiquidityPoolAsset}
   */
  return _createClass(LiquidityPoolAsset, [{
    key: "toXDRObject",
    value:
    /**
     * Returns the `xdr.ChangeTrustAsset` object for this liquidity pool asset.
     *
     * Note: To convert from an {@link Asset `Asset`} to `xdr.ChangeTrustAsset`
     * please refer to the
     * {@link Asset.toChangeTrustXDRObject `Asset.toChangeTrustXDRObject`} method.
     *
     * @returns {xdr.ChangeTrustAsset} XDR ChangeTrustAsset object.
     */
    function toXDRObject() {
      var lpConstantProductParamsXdr = new _xdr["default"].LiquidityPoolConstantProductParameters({
        assetA: this.assetA.toXDRObject(),
        assetB: this.assetB.toXDRObject(),
        fee: this.fee
      });
      var lpParamsXdr = new _xdr["default"].LiquidityPoolParameters('liquidityPoolConstantProduct', lpConstantProductParamsXdr);
      return new _xdr["default"].ChangeTrustAsset('assetTypePoolShare', lpParamsXdr);
    }

    /**
     * @returns {LiquidityPoolParameters} Liquidity pool parameters.
     */
  }, {
    key: "getLiquidityPoolParameters",
    value: function getLiquidityPoolParameters() {
      return _objectSpread(_objectSpread({}, this), {}, {
        assetA: this.assetA,
        assetB: this.assetB,
        fee: this.fee
      });
    }

    /**
     * @see [Assets concept](https://developers.stellar.org/docs/glossary/assets/)
     * @returns {AssetType.liquidityPoolShares} asset type. Can only be `liquidity_pool_shares`.
     */
  }, {
    key: "getAssetType",
    value: function getAssetType() {
      return 'liquidity_pool_shares';
    }

    /**
     * @param {LiquidityPoolAsset} other the LiquidityPoolAsset to compare
     * @returns {boolean} `true` if this asset equals the given asset.
     */
  }, {
    key: "equals",
    value: function equals(other) {
      return this.assetA.equals(other.assetA) && this.assetB.equals(other.assetB) && this.fee === other.fee;
    }
  }, {
    key: "toString",
    value: function toString() {
      var poolId = (0, _get_liquidity_pool_id.getLiquidityPoolId)('constant_product', this.getLiquidityPoolParameters()).toString('hex');
      return "liquidity_pool:".concat(poolId);
    }
  }], [{
    key: "fromOperation",
    value: function fromOperation(ctAssetXdr) {
      var assetType = ctAssetXdr["switch"]();
      if (assetType === _xdr["default"].AssetType.assetTypePoolShare()) {
        var liquidityPoolParameters = ctAssetXdr.liquidityPool().constantProduct();
        return new this(_asset.Asset.fromOperation(liquidityPoolParameters.assetA()), _asset.Asset.fromOperation(liquidityPoolParameters.assetB()), liquidityPoolParameters.fee());
      }
      throw new Error("Invalid asset type: ".concat(assetType.name));
    }
  }]);
}();