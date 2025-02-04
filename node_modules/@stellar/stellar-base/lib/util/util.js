"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.trimEnd = void 0;
var trimEnd = exports.trimEnd = function trimEnd(input, _char) {
  var isNumber = typeof input === 'number';
  var str = String(input);
  while (str.endsWith(_char)) {
    str = str.slice(0, -1);
  }
  return isNumber ? Number(str) : str;
};