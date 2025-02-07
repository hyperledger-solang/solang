"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.hash = hash;
var _sha = require("sha.js");
function hash(data) {
  var hasher = new _sha.sha256();
  hasher.update(data, 'utf8');
  return hasher.digest();
}