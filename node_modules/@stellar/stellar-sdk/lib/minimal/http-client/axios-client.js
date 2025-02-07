"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.create = exports.axiosClient = void 0;
var _axios = _interopRequireDefault(require("axios"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
var axiosClient = exports.axiosClient = _axios.default;
var create = exports.create = _axios.default.create;