"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
var _exportNames = {
  Server: true,
  FEDERATION_RESPONSE_MAX_SIZE: true
};
Object.defineProperty(exports, "FEDERATION_RESPONSE_MAX_SIZE", {
  enumerable: true,
  get: function get() {
    return _server.FEDERATION_RESPONSE_MAX_SIZE;
  }
});
Object.defineProperty(exports, "Server", {
  enumerable: true,
  get: function get() {
    return _server.FederationServer;
  }
});
var _server = require("./server");
var _api = require("./api");
Object.keys(_api).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _api[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function get() {
      return _api[key];
    }
  });
});