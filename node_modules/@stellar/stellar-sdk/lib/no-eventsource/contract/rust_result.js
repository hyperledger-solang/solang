"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Ok = exports.Err = void 0;
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var Ok = exports.Ok = function () {
  function Ok(value) {
    _classCallCheck(this, Ok);
    this.value = value;
  }
  return _createClass(Ok, [{
    key: "unwrapErr",
    value: function unwrapErr() {
      throw new Error("No error");
    }
  }, {
    key: "unwrap",
    value: function unwrap() {
      return this.value;
    }
  }, {
    key: "isOk",
    value: function isOk() {
      return true;
    }
  }, {
    key: "isErr",
    value: function isErr() {
      return false;
    }
  }]);
}();
var Err = exports.Err = function () {
  function Err(error) {
    _classCallCheck(this, Err);
    this.error = error;
  }
  return _createClass(Err, [{
    key: "unwrapErr",
    value: function unwrapErr() {
      return this.error;
    }
  }, {
    key: "unwrap",
    value: function unwrap() {
      throw new Error(this.error.message);
    }
  }, {
    key: "isOk",
    value: function isOk() {
      return false;
    }
  }, {
    key: "isErr",
    value: function isErr() {
      return true;
    }
  }]);
}();