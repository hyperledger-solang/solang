"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.verifyChecksum = verifyChecksum;
function verifyChecksum(expected, actual) {
  if (expected.length !== actual.length) {
    return false;
  }
  if (expected.length === 0) {
    return true;
  }
  for (var i = 0; i < expected.length; i += 1) {
    if (expected[i] !== actual[i]) {
      return false;
    }
  }
  return true;
}