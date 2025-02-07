"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports["default"] = void 0;
var _bignumber = _interopRequireDefault(require("bignumber.js"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
var BigNumber = _bignumber["default"].clone();
BigNumber.DEBUG = true; // gives us exceptions on bad constructor values
var _default = exports["default"] = BigNumber;