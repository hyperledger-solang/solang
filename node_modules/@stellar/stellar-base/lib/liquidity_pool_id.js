"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.LiquidityPoolId = void 0;
var _xdr = _interopRequireDefault(require("./xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * LiquidityPoolId class represents the asset referenced by a trustline to a
 * liquidity pool.
 *
 * @constructor
 * @param {string} liquidityPoolId - The ID of the liquidity pool in string 'hex'.
 */
var LiquidityPoolId = exports.LiquidityPoolId = /*#__PURE__*/function () {
  function LiquidityPoolId(liquidityPoolId) {
    _classCallCheck(this, LiquidityPoolId);
    if (!liquidityPoolId) {
      throw new Error('liquidityPoolId cannot be empty');
    }
    if (!/^[a-f0-9]{64}$/.test(liquidityPoolId)) {
      throw new Error('Liquidity pool ID is not a valid hash');
    }
    this.liquidityPoolId = liquidityPoolId;
  }

  /**
   * Returns a liquidity pool ID object from its xdr.TrustLineAsset representation.
   * @param {xdr.TrustLineAsset} tlAssetXdr - The asset XDR object.
   * @returns {LiquidityPoolId}
   */
  return _createClass(LiquidityPoolId, [{
    key: "toXDRObject",
    value:
    /**
     * Returns the `xdr.TrustLineAsset` object for this liquidity pool ID.
     *
     * Note: To convert from {@link Asset `Asset`} to `xdr.TrustLineAsset` please
     * refer to the
     * {@link Asset.toTrustLineXDRObject `Asset.toTrustLineXDRObject`} method.
     *
     * @returns {xdr.TrustLineAsset} XDR LiquidityPoolId object
     */
    function toXDRObject() {
      var xdrPoolId = _xdr["default"].PoolId.fromXDR(this.liquidityPoolId, 'hex');
      return new _xdr["default"].TrustLineAsset('assetTypePoolShare', xdrPoolId);
    }

    /**
     * @returns {string} Liquidity pool ID.
     */
  }, {
    key: "getLiquidityPoolId",
    value: function getLiquidityPoolId() {
      return String(this.liquidityPoolId);
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
     * @param {LiquidityPoolId} asset LiquidityPoolId to compare.
     * @returns {boolean} `true` if this asset equals the given asset.
     */
  }, {
    key: "equals",
    value: function equals(asset) {
      return this.liquidityPoolId === asset.getLiquidityPoolId();
    }
  }, {
    key: "toString",
    value: function toString() {
      return "liquidity_pool:".concat(this.liquidityPoolId);
    }
  }], [{
    key: "fromOperation",
    value: function fromOperation(tlAssetXdr) {
      var assetType = tlAssetXdr["switch"]();
      if (assetType === _xdr["default"].AssetType.assetTypePoolShare()) {
        var liquidityPoolId = tlAssetXdr.liquidityPoolId().toString('hex');
        return new this(liquidityPoolId);
      }
      throw new Error("Invalid asset type: ".concat(assetType.name));
    }
  }]);
}();