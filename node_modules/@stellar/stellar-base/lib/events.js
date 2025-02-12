"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.humanizeEvents = humanizeEvents;
var _strkey = require("./strkey");
var _scval = require("./scval");
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * Converts raw diagnostic or contract events into something with a flatter,
 * human-readable, and understandable structure.
 *
 * @param {xdr.DiagnosticEvent[] | xdr.ContractEvent[]} events  either contract
 *    events or diagnostic events to parse into a friendly format
 *
 * @returns {SorobanEvent[]}  a list of human-readable event structures, where
 *    each element has the following properties:
 *  - type: a string of one of 'system', 'contract', 'diagnostic
 *  - contractId?: optionally, a `C...` encoded strkey
 *  - topics: a list of {@link scValToNative} invocations on the topics
 *  - data: similarly, a {@link scValToNative} invocation on the raw event data
 */
function humanizeEvents(events) {
  return events.map(function (e) {
    // A pseudo-instanceof check for xdr.DiagnosticEvent more reliable
    // in mixed SDK environments:
    if (e.inSuccessfulContractCall) {
      return extractEvent(e.event());
    }
    return extractEvent(e);
  });
}
function extractEvent(event) {
  return _objectSpread(_objectSpread({}, typeof event.contractId === 'function' && event.contractId() != null && {
    contractId: _strkey.StrKey.encodeContract(event.contractId())
  }), {}, {
    type: event.type().name,
    topics: event.body().value().topics().map(function (t) {
      return (0, _scval.scValToNative)(t);
    }),
    data: (0, _scval.scValToNative)(event.body().value().data())
  });
}