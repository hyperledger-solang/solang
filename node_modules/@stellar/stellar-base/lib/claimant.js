"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Claimant = void 0;
var _xdr = _interopRequireDefault(require("./xdr"));
var _keypair = require("./keypair");
var _strkey = require("./strkey");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * Claimant class represents an xdr.Claimant
 *
 * The claim predicate is optional, it defaults to unconditional if none is specified.
 *
 * @constructor
 * @param {string} destination - The destination account ID.
 * @param {xdr.ClaimPredicate} [predicate] - The claim predicate.
 */
var Claimant = exports.Claimant = /*#__PURE__*/function () {
  function Claimant(destination, predicate) {
    _classCallCheck(this, Claimant);
    if (destination && !_strkey.StrKey.isValidEd25519PublicKey(destination)) {
      throw new Error('Destination is invalid');
    }
    this._destination = destination;
    if (!predicate) {
      this._predicate = _xdr["default"].ClaimPredicate.claimPredicateUnconditional();
    } else if (predicate instanceof _xdr["default"].ClaimPredicate) {
      this._predicate = predicate;
    } else {
      throw new Error('Predicate should be an xdr.ClaimPredicate');
    }
  }

  /**
   * Returns an unconditional claim predicate
   * @Return {xdr.ClaimPredicate}
   */
  return _createClass(Claimant, [{
    key: "toXDRObject",
    value:
    /**
     * Returns the xdr object for this claimant.
     * @returns {xdr.Claimant} XDR Claimant object
     */
    function toXDRObject() {
      var claimant = new _xdr["default"].ClaimantV0({
        destination: _keypair.Keypair.fromPublicKey(this._destination).xdrAccountId(),
        predicate: this._predicate
      });
      return _xdr["default"].Claimant.claimantTypeV0(claimant);
    }

    /**
     * @type {string}
     * @readonly
     */
  }, {
    key: "destination",
    get: function get() {
      return this._destination;
    },
    set: function set(value) {
      throw new Error('Claimant is immutable');
    }

    /**
     * @type {xdr.ClaimPredicate}
     * @readonly
     */
  }, {
    key: "predicate",
    get: function get() {
      return this._predicate;
    },
    set: function set(value) {
      throw new Error('Claimant is immutable');
    }
  }], [{
    key: "predicateUnconditional",
    value: function predicateUnconditional() {
      return _xdr["default"].ClaimPredicate.claimPredicateUnconditional();
    }

    /**
     * Returns an `and` claim predicate
     * @param {xdr.ClaimPredicate} left an xdr.ClaimPredicate
     * @param {xdr.ClaimPredicate} right an xdr.ClaimPredicate
     * @Return {xdr.ClaimPredicate}
     */
  }, {
    key: "predicateAnd",
    value: function predicateAnd(left, right) {
      if (!(left instanceof _xdr["default"].ClaimPredicate)) {
        throw new Error('left Predicate should be an xdr.ClaimPredicate');
      }
      if (!(right instanceof _xdr["default"].ClaimPredicate)) {
        throw new Error('right Predicate should be an xdr.ClaimPredicate');
      }
      return _xdr["default"].ClaimPredicate.claimPredicateAnd([left, right]);
    }

    /**
     * Returns an `or` claim predicate
     * @param {xdr.ClaimPredicate} left an xdr.ClaimPredicate
     * @param {xdr.ClaimPredicate} right an xdr.ClaimPredicate
     * @Return {xdr.ClaimPredicate}
     */
  }, {
    key: "predicateOr",
    value: function predicateOr(left, right) {
      if (!(left instanceof _xdr["default"].ClaimPredicate)) {
        throw new Error('left Predicate should be an xdr.ClaimPredicate');
      }
      if (!(right instanceof _xdr["default"].ClaimPredicate)) {
        throw new Error('right Predicate should be an xdr.ClaimPredicate');
      }
      return _xdr["default"].ClaimPredicate.claimPredicateOr([left, right]);
    }

    /**
     * Returns a `not` claim predicate
     * @param {xdr.ClaimPredicate} predicate an xdr.ClaimPredicate
     * @Return {xdr.ClaimPredicate}
     */
  }, {
    key: "predicateNot",
    value: function predicateNot(predicate) {
      if (!(predicate instanceof _xdr["default"].ClaimPredicate)) {
        throw new Error('right Predicate should be an xdr.ClaimPredicate');
      }
      return _xdr["default"].ClaimPredicate.claimPredicateNot(predicate);
    }

    /**
     * Returns a `BeforeAbsoluteTime` claim predicate
     *
     * This predicate will be fulfilled if the closing time of the ledger that
     * includes the CreateClaimableBalance operation is less than this (absolute)
     * Unix timestamp (expressed in seconds).
     *
     * @param {string} absBefore Unix epoch (in seconds) as a string
     * @Return {xdr.ClaimPredicate}
     */
  }, {
    key: "predicateBeforeAbsoluteTime",
    value: function predicateBeforeAbsoluteTime(absBefore) {
      return _xdr["default"].ClaimPredicate.claimPredicateBeforeAbsoluteTime(_xdr["default"].Int64.fromString(absBefore));
    }

    /**
     * Returns a `BeforeRelativeTime` claim predicate
     *
     * This predicate will be fulfilled if the closing time of the ledger that
     * includes the CreateClaimableBalance operation plus this relative time delta
     * (in seconds) is less than the current time.
     *
     * @param {strings} seconds seconds since closeTime of the ledger in which the ClaimableBalanceEntry was created (as string)
     * @Return {xdr.ClaimPredicate}
     */
  }, {
    key: "predicateBeforeRelativeTime",
    value: function predicateBeforeRelativeTime(seconds) {
      return _xdr["default"].ClaimPredicate.claimPredicateBeforeRelativeTime(_xdr["default"].Int64.fromString(seconds));
    }

    /**
     * Returns a claimant object from its XDR object representation.
     * @param {xdr.Claimant} claimantXdr - The claimant xdr object.
     * @returns {Claimant}
     */
  }, {
    key: "fromXDR",
    value: function fromXDR(claimantXdr) {
      var value;
      switch (claimantXdr["switch"]()) {
        case _xdr["default"].ClaimantType.claimantTypeV0():
          value = claimantXdr.v0();
          return new this(_strkey.StrKey.encodeEd25519PublicKey(value.destination().ed25519()), value.predicate());
        default:
          throw new Error("Invalid claimant type: ".concat(claimantXdr["switch"]().name));
      }
    }
  }]);
}();