"use strict";

// Module dependencies.
var base32 = require("./base32");


// Wrap decoder finalize to return a buffer;
var finalizeDecode = base32.Decoder.prototype.finalize;
base32.Decoder.prototype.finalize = function (buf) {
  var bytes = finalizeDecode.call(this, buf);
  return new Buffer(bytes);
};


// Export Base32.
module.exports = base32;
