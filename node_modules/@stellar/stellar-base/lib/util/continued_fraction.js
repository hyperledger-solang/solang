"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.best_r = best_r;
var _bignumber = _interopRequireDefault(require("./bignumber"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t["return"] && (u = t["return"](), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
// eslint-disable-next-line no-bitwise
var MAX_INT = (1 << 31 >>> 0) - 1;

/**
 * Calculates and returns the best rational approximation of the given real number.
 * @private
 * @param {string|number|BigNumber} rawNumber Real number
 * @throws Error Throws `Error` when the best rational approximation cannot be found.
 * @returns {array} first element is n (numerator), second element is d (denominator)
 */
function best_r(rawNumber) {
  var number = new _bignumber["default"](rawNumber);
  var a;
  var f;
  var fractions = [[new _bignumber["default"](0), new _bignumber["default"](1)], [new _bignumber["default"](1), new _bignumber["default"](0)]];
  var i = 2;

  // eslint-disable-next-line no-constant-condition
  while (true) {
    if (number.gt(MAX_INT)) {
      break;
    }
    a = number.integerValue(_bignumber["default"].ROUND_FLOOR);
    f = number.minus(a);
    var h = a.times(fractions[i - 1][0]).plus(fractions[i - 2][0]);
    var k = a.times(fractions[i - 1][1]).plus(fractions[i - 2][1]);
    if (h.gt(MAX_INT) || k.gt(MAX_INT)) {
      break;
    }
    fractions.push([h, k]);
    if (f.eq(0)) {
      break;
    }
    number = new _bignumber["default"](1).div(f);
    i += 1;
  }
  var _fractions = _slicedToArray(fractions[fractions.length - 1], 2),
    n = _fractions[0],
    d = _fractions[1];
  if (n.isZero() || d.isZero()) {
    throw new Error("Couldn't find approximation");
  }
  return [n.toNumber(), d.toNumber()];
}