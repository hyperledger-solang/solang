"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Soroban = void 0;
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _toArray(r) { return _arrayWithHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray(r) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArray(r) { if ("undefined" != typeof Symbol && null != r[Symbol.iterator] || null != r["@@iterator"]) return Array.from(r); }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/* Helper class to assist with formatting and parsing token amounts. */
var Soroban = exports.Soroban = /*#__PURE__*/function () {
  function Soroban() {
    _classCallCheck(this, Soroban);
  }
  return _createClass(Soroban, null, [{
    key: "formatTokenAmount",
    value:
    /**
     * Given a whole number smart contract amount of a token and an amount of
     * decimal places (if the token has any), it returns a "display" value.
     *
     * All arithmetic inside the contract is performed on integers to avoid
     * potential precision and consistency issues of floating-point.
     *
     * @param {string} amount   the token amount you want to display
     * @param {number} decimals specify how many decimal places a token has
     *
     * @returns {string} the display value
     * @throws {TypeError} if the given amount has a decimal point already
     * @example
     * formatTokenAmount("123000", 4) === "12.3";
     */
    function formatTokenAmount(amount, decimals) {
      if (amount.includes('.')) {
        throw new TypeError('No decimals are allowed');
      }
      var formatted = amount;
      if (decimals > 0) {
        if (decimals > formatted.length) {
          formatted = ['0', formatted.toString().padStart(decimals, '0')].join('.');
        } else {
          formatted = [formatted.slice(0, -decimals), formatted.slice(-decimals)].join('.');
        }
      }

      // remove trailing zero if any
      return formatted.replace(/(\.\d*?)0+$/, '$1');
    }

    /**
     * Parse a token amount to use it on smart contract
     *
     * This function takes the display value and its decimals (if the token has
     * any) and returns a string that'll be used within the smart contract.
     *
     * @param {string} value      the token amount you want to use it on smart
     *    contract which you've been displaying in a UI
     * @param {number} decimals   the number of decimal places expected in the
     *    display value (different than the "actual" number, because suffix zeroes
     *    might not be present)
     *
     * @returns {string}  the whole number token amount represented by the display
     *    value with the decimal places shifted over
     *
     * @example
     * const displayValueAmount = "123.4560"
     * const parsedAmtForSmartContract = parseTokenAmount(displayValueAmount, 5);
     * parsedAmtForSmartContract === "12345600"
     */
  }, {
    key: "parseTokenAmount",
    value: function parseTokenAmount(value, decimals) {
      var _fraction$padEnd;
      var _value$split$slice = value.split('.').slice(),
        _value$split$slice2 = _toArray(_value$split$slice),
        whole = _value$split$slice2[0],
        fraction = _value$split$slice2[1],
        rest = _value$split$slice2.slice(2);
      if (rest.length) {
        throw new Error("Invalid decimal value: ".concat(value));
      }
      var shifted = BigInt(whole + ((_fraction$padEnd = fraction === null || fraction === void 0 ? void 0 : fraction.padEnd(decimals, '0')) !== null && _fraction$padEnd !== void 0 ? _fraction$padEnd : '0'.repeat(decimals)));
      return shifted.toString();
    }
  }]);
}();