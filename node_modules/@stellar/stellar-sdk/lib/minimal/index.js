"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
Object.defineProperty(exports, "__esModule", {
  value: true
});
var _exportNames = {
  Config: true,
  Utils: true,
  StellarToml: true,
  Federation: true,
  WebAuth: true,
  Friendbot: true,
  Horizon: true,
  rpc: true,
  contract: true
};
Object.defineProperty(exports, "Config", {
  enumerable: true,
  get: function get() {
    return _config.Config;
  }
});
exports.StellarToml = exports.Horizon = exports.Friendbot = exports.Federation = void 0;
Object.defineProperty(exports, "Utils", {
  enumerable: true,
  get: function get() {
    return _utils.Utils;
  }
});
exports.rpc = exports.default = exports.contract = exports.WebAuth = void 0;
var _errors = require("./errors");
Object.keys(_errors).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _errors[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function get() {
      return _errors[key];
    }
  });
});
var _config = require("./config");
var _utils = require("./utils");
var _StellarToml = _interopRequireWildcard(require("./stellartoml"));
exports.StellarToml = _StellarToml;
var _Federation = _interopRequireWildcard(require("./federation"));
exports.Federation = _Federation;
var _WebAuth = _interopRequireWildcard(require("./webauth"));
exports.WebAuth = _WebAuth;
var _Friendbot = _interopRequireWildcard(require("./friendbot"));
exports.Friendbot = _Friendbot;
var _Horizon = _interopRequireWildcard(require("./horizon"));
exports.Horizon = _Horizon;
var _rpc = _interopRequireWildcard(require("./rpc"));
exports.rpc = _rpc;
var _contract = _interopRequireWildcard(require("./contract"));
exports.contract = _contract;
var _stellarBase = require("@stellar/stellar-base");
Object.keys(_stellarBase).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _stellarBase[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function get() {
      return _stellarBase[key];
    }
  });
});
function _getRequireWildcardCache(e) { if ("function" != typeof WeakMap) return null; var r = new WeakMap(), t = new WeakMap(); return (_getRequireWildcardCache = function _getRequireWildcardCache(e) { return e ? t : r; })(e); }
function _interopRequireWildcard(e, r) { if (!r && e && e.__esModule) return e; if (null === e || "object" != _typeof(e) && "function" != typeof e) return { default: e }; var t = _getRequireWildcardCache(r); if (t && t.has(e)) return t.get(e); var n = { __proto__: null }, a = Object.defineProperty && Object.getOwnPropertyDescriptor; for (var u in e) if ("default" !== u && {}.hasOwnProperty.call(e, u)) { var i = a ? Object.getOwnPropertyDescriptor(e, u) : null; i && (i.get || i.set) ? Object.defineProperty(n, u, i) : n[u] = e[u]; } return n.default = e, t && t.set(e, n), n; }
var _default = exports.default = module.exports;
if (typeof global.__USE_AXIOS__ === 'undefined') {
  global.__USE_AXIOS__ = true;
}
if (typeof global.__USE_EVENTSOURCE__ === 'undefined') {
  global.__USE_EVENTSOURCE__ = false;
}