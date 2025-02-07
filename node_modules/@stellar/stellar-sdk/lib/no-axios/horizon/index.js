"use strict";

function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
Object.defineProperty(exports, "__esModule", {
  value: true
});
var _exportNames = {
  Server: true,
  AxiosClient: true,
  SERVER_TIME_MAP: true,
  getCurrentServerTime: true
};
Object.defineProperty(exports, "AxiosClient", {
  enumerable: true,
  get: function get() {
    return _horizon_axios_client.default;
  }
});
Object.defineProperty(exports, "SERVER_TIME_MAP", {
  enumerable: true,
  get: function get() {
    return _horizon_axios_client.SERVER_TIME_MAP;
  }
});
Object.defineProperty(exports, "Server", {
  enumerable: true,
  get: function get() {
    return _server.HorizonServer;
  }
});
exports.default = void 0;
Object.defineProperty(exports, "getCurrentServerTime", {
  enumerable: true,
  get: function get() {
    return _horizon_axios_client.getCurrentServerTime;
  }
});
var _horizon_api = require("./horizon_api");
Object.keys(_horizon_api).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _horizon_api[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function get() {
      return _horizon_api[key];
    }
  });
});
var _server_api = require("./server_api");
Object.keys(_server_api).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _server_api[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function get() {
      return _server_api[key];
    }
  });
});
var _account_response = require("./account_response");
Object.keys(_account_response).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _account_response[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function get() {
      return _account_response[key];
    }
  });
});
var _server = require("./server");
var _horizon_axios_client = _interopRequireWildcard(require("./horizon_axios_client"));
function _getRequireWildcardCache(e) { if ("function" != typeof WeakMap) return null; var r = new WeakMap(), t = new WeakMap(); return (_getRequireWildcardCache = function _getRequireWildcardCache(e) { return e ? t : r; })(e); }
function _interopRequireWildcard(e, r) { if (!r && e && e.__esModule) return e; if (null === e || "object" != _typeof(e) && "function" != typeof e) return { default: e }; var t = _getRequireWildcardCache(r); if (t && t.has(e)) return t.get(e); var n = { __proto__: null }, a = Object.defineProperty && Object.getOwnPropertyDescriptor; for (var u in e) if ("default" !== u && {}.hasOwnProperty.call(e, u)) { var i = a ? Object.getOwnPropertyDescriptor(e, u) : null; i && (i.get || i.set) ? Object.defineProperty(n, u, i) : n[u] = e[u]; } return n.default = e, t && t.set(e, n), n; }
var _default = exports.default = module.exports;