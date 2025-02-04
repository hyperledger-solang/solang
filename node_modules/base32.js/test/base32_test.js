"use strict";

var assert = require("assert");
var base32 = require("..");
var fixtures = require("./fixtures");

describe("Decoder", function () {

  fixtures.forEach(function (subject) {
    var test = subject.buf;

    subject.rfc4648.forEach(function (str) {
      it("should decode rfc4648 " + str, function () {
        var decoder = new base32.Decoder({ type: "rfc4648" });
        var decoded = decoder.write(str).finalize();
        compare(decoded, test);
        var s = new base32.Decoder().write(str).finalize();
        compare(s, test);
      });
    });

    subject.crock32.forEach(function (str) {
      it("should decode crock32 " + str, function () {
        var decoder = new base32.Decoder({ type: "crockford" });
        var decoded = decoder.write(str).finalize();
        compare(decoded, test);
      });
    });

    subject.base32hex.forEach(function (str) {
      it("should decode base32hex " + str, function () {
        var decoder = new base32.Decoder({ type: "base32hex" });
        var decoded = decoder.write(str).finalize();
        compare(decoded, test);
      });
    });

  });

});

describe("Encoder", function () {

  fixtures.forEach(function (subject) {
    var buf = subject.buf;

    it("should encode rfc4648 " + buf, function () {
      var test = subject.rfc4648[0];
      var encoder = new base32.Encoder({ type: "rfc4648" });
      var encode = encoder.write(buf).finalize();
      assert.equal(encode, test);
      var s = new base32.Encoder().write(buf).finalize();
      assert.equal(s, test);
    });

    it("should encode crock32 " + buf, function () {
      var test = subject.crock32[0];
      var encoder = new base32.Encoder({ type: "crockford" });
      var encoded = encoder.write(buf).finalize();
      assert.equal(encoded, test);
    });

    it("should encode crock32 " + buf + " with lower case", function () {
      var test = subject.crock32[0];
      var encoder = new base32.Encoder({ type: "crockford", lc: true });
      var encoded = encoder.write(buf).finalize();
      assert.equal(encoded, test.toLowerCase());
    });

    it("should encode base32hex " + buf + " with lower case", function () {
      var test = subject.base32hex[0];
      var encoder = new base32.Encoder({ type: "base32hex", lc: true });
      var encoded = encoder.write(buf).finalize();
      assert.equal(encoded, test.toLowerCase());
    });

  });

});

function compare (a, b) {
  if (typeof Buffer != "undefined") {
    b = new Buffer(b);
    return assert.strictEqual(b.compare(a), 0);
  }
  assert.deepEqual(a, b);
}
