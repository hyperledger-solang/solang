"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
var _exportNames = {
  httpClient: true,
  create: true
};
exports.httpClient = exports.create = void 0;
var _types = require("./types");
Object.keys(_types).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _types[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function get() {
      return _types[key];
    }
  });
});
var httpClient;
var create;
if (false) {
  var axiosModule = require('./axios-client');
  exports.httpClient = httpClient = axiosModule.axiosClient;
  exports.create = create = axiosModule.create;
} else {
  var fetchModule = require('./fetch-client');
  exports.httpClient = httpClient = fetchModule.fetchClient;
  exports.create = create = fetchModule.create;
}