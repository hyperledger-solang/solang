"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.TradeAggregationCallBuilder = void 0;
var _call_builder = require("./call_builder");
var _errors = require("../errors");
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
var allowedResolutions = [60000, 300000, 900000, 3600000, 86400000, 604800000];
var TradeAggregationCallBuilder = exports.TradeAggregationCallBuilder = function (_CallBuilder) {
  function TradeAggregationCallBuilder(serverUrl, base, counter, start_time, end_time, resolution, offset) {
    var _this;
    _classCallCheck(this, TradeAggregationCallBuilder);
    _this = _callSuper(this, TradeAggregationCallBuilder, [serverUrl]);
    _this.url.segment("trade_aggregations");
    if (!base.isNative()) {
      _this.url.setQuery("base_asset_type", base.getAssetType());
      _this.url.setQuery("base_asset_code", base.getCode());
      _this.url.setQuery("base_asset_issuer", base.getIssuer());
    } else {
      _this.url.setQuery("base_asset_type", "native");
    }
    if (!counter.isNative()) {
      _this.url.setQuery("counter_asset_type", counter.getAssetType());
      _this.url.setQuery("counter_asset_code", counter.getCode());
      _this.url.setQuery("counter_asset_issuer", counter.getIssuer());
    } else {
      _this.url.setQuery("counter_asset_type", "native");
    }
    if (typeof start_time !== "number" || typeof end_time !== "number") {
      throw new _errors.BadRequestError("Invalid time bounds", [start_time, end_time]);
    } else {
      _this.url.setQuery("start_time", start_time.toString());
      _this.url.setQuery("end_time", end_time.toString());
    }
    if (!_this.isValidResolution(resolution)) {
      throw new _errors.BadRequestError("Invalid resolution", resolution);
    } else {
      _this.url.setQuery("resolution", resolution.toString());
    }
    if (!_this.isValidOffset(offset, resolution)) {
      throw new _errors.BadRequestError("Invalid offset", offset);
    } else {
      _this.url.setQuery("offset", offset.toString());
    }
    return _this;
  }
  _inherits(TradeAggregationCallBuilder, _CallBuilder);
  return _createClass(TradeAggregationCallBuilder, [{
    key: "isValidResolution",
    value: function isValidResolution(resolution) {
      return allowedResolutions.some(function (allowed) {
        return allowed === resolution;
      });
    }
  }, {
    key: "isValidOffset",
    value: function isValidOffset(offset, resolution) {
      var hour = 3600000;
      return !(offset > resolution || offset >= 24 * hour || offset % hour !== 0);
    }
  }]);
}(_call_builder.CallBuilder);