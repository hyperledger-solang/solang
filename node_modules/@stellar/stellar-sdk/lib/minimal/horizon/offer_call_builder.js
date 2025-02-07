"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.OfferCallBuilder = void 0;
var _call_builder = require("./call_builder");
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
function _callSuper(t, o, e) { return o = _getPrototypeOf(o), _possibleConstructorReturn(t, _isNativeReflectConstruct() ? Reflect.construct(o, e || [], _getPrototypeOf(t).constructor) : o.apply(t, e)); }
function _possibleConstructorReturn(t, e) { if (e && ("object" == _typeof(e) || "function" == typeof e)) return e; if (void 0 !== e) throw new TypeError("Derived constructors may only return object or undefined"); return _assertThisInitialized(t); }
function _assertThisInitialized(e) { if (void 0 === e) throw new ReferenceError("this hasn't been initialised - super() hasn't been called"); return e; }
function _isNativeReflectConstruct() { try { var t = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function () {})); } catch (t) {} return (_isNativeReflectConstruct = function _isNativeReflectConstruct() { return !!t; })(); }
function _getPrototypeOf(t) { return _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf.bind() : function (t) { return t.__proto__ || Object.getPrototypeOf(t); }, _getPrototypeOf(t); }
function _inherits(t, e) { if ("function" != typeof e && null !== e) throw new TypeError("Super expression must either be null or a function"); t.prototype = Object.create(e && e.prototype, { constructor: { value: t, writable: !0, configurable: !0 } }), Object.defineProperty(t, "prototype", { writable: !1 }), e && _setPrototypeOf(t, e); }
function _setPrototypeOf(t, e) { return _setPrototypeOf = Object.setPrototypeOf ? Object.setPrototypeOf.bind() : function (t, e) { return t.__proto__ = e, t; }, _setPrototypeOf(t, e); }
var OfferCallBuilder = exports.OfferCallBuilder = function (_CallBuilder) {
  function OfferCallBuilder(serverUrl) {
    var _this;
    _classCallCheck(this, OfferCallBuilder);
    _this = _callSuper(this, OfferCallBuilder, [serverUrl, "offers"]);
    _this.url.segment("offers");
    return _this;
  }
  _inherits(OfferCallBuilder, _CallBuilder);
  return _createClass(OfferCallBuilder, [{
    key: "offer",
    value: function offer(offerId) {
      var builder = new _call_builder.CallBuilder(this.url.clone());
      builder.filter.push([offerId]);
      return builder;
    }
  }, {
    key: "forAccount",
    value: function forAccount(id) {
      return this.forEndpoint("accounts", id);
    }
  }, {
    key: "buying",
    value: function buying(asset) {
      if (!asset.isNative()) {
        this.url.setQuery("buying_asset_type", asset.getAssetType());
        this.url.setQuery("buying_asset_code", asset.getCode());
        this.url.setQuery("buying_asset_issuer", asset.getIssuer());
      } else {
        this.url.setQuery("buying_asset_type", "native");
      }
      return this;
    }
  }, {
    key: "selling",
    value: function selling(asset) {
      if (!asset.isNative()) {
        this.url.setQuery("selling_asset_type", asset.getAssetType());
        this.url.setQuery("selling_asset_code", asset.getCode());
        this.url.setQuery("selling_asset_issuer", asset.getIssuer());
      } else {
        this.url.setQuery("selling_asset_type", "native");
      }
      return this;
    }
  }, {
    key: "sponsor",
    value: function sponsor(id) {
      this.url.setQuery("sponsor", id);
      return this;
    }
  }, {
    key: "seller",
    value: function seller(_seller) {
      this.url.setQuery("seller", _seller);
      return this;
    }
  }]);
}(_call_builder.CallBuilder);