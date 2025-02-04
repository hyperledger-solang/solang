"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.SorobanDataBuilder = void 0;
var _xdr = _interopRequireDefault(require("./xdr"));
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * Supports building {@link xdr.SorobanTransactionData} structures with various
 * items set to specific values.
 *
 * This is recommended for when you are building
 * {@link Operation.extendFootprintTtl} / {@link Operation.restoreFootprint}
 * operations and need to {@link TransactionBuilder.setSorobanData} to avoid
 * (re)building the entire data structure from scratch.
 *
 * @constructor
 *
 * @param {string | xdr.SorobanTransactionData} [sorobanData]  either a
 *      base64-encoded string that represents an
 *      {@link xdr.SorobanTransactionData} instance or an XDR instance itself
 *      (it will be copied); if omitted or "falsy" (e.g. an empty string), it
 *      starts with an empty instance
 *
 * @example
 * // You want to use an existing data blob but override specific parts.
 * const newData = new SorobanDataBuilder(existing)
 *   .setReadOnly(someLedgerKeys)
 *   .setRefundableFee("1000")
 *   .build();
 *
 * // You want an instance from scratch
 * const newData = new SorobanDataBuilder()
 *   .setFootprint([someLedgerKey], [])
 *   .setRefundableFee("1000")
 *   .build();
 */
var SorobanDataBuilder = exports.SorobanDataBuilder = /*#__PURE__*/function () {
  function SorobanDataBuilder(sorobanData) {
    _classCallCheck(this, SorobanDataBuilder);
    _defineProperty(this, "_data", void 0);
    var data;
    if (!sorobanData) {
      data = new _xdr["default"].SorobanTransactionData({
        resources: new _xdr["default"].SorobanResources({
          footprint: new _xdr["default"].LedgerFootprint({
            readOnly: [],
            readWrite: []
          }),
          instructions: 0,
          readBytes: 0,
          writeBytes: 0
        }),
        ext: new _xdr["default"].ExtensionPoint(0),
        resourceFee: new _xdr["default"].Int64(0)
      });
    } else if (typeof sorobanData === 'string' || ArrayBuffer.isView(sorobanData)) {
      data = SorobanDataBuilder.fromXDR(sorobanData);
    } else {
      data = SorobanDataBuilder.fromXDR(sorobanData.toXDR()); // copy
    }
    this._data = data;
  }

  /**
   * Decodes and builds a {@link xdr.SorobanTransactionData} instance.
   * @param {Uint8Array|Buffer|string} data   raw input to decode
   * @returns {xdr.SorobanTransactionData}
   */
  return _createClass(SorobanDataBuilder, [{
    key: "setResourceFee",
    value:
    /**
     * Sets the resource fee portion of the Soroban data.
     * @param {number | bigint | string} fee  the resource fee to set (int64)
     * @returns {SorobanDataBuilder}
     */
    function setResourceFee(fee) {
      this._data.resourceFee(new _xdr["default"].Int64(fee));
      return this;
    }

    /**
     * Sets up the resource metrics.
     *
     * You should almost NEVER need this, as its often generated / provided to you
     * by transaction simulation/preflight from a Soroban RPC server.
     *
     * @param {number} cpuInstrs      number of CPU instructions
     * @param {number} readBytes      number of bytes being read
     * @param {number} writeBytes     number of bytes being written
     *
     * @returns {SorobanDataBuilder}
     */
  }, {
    key: "setResources",
    value: function setResources(cpuInstrs, readBytes, writeBytes) {
      this._data.resources().instructions(cpuInstrs);
      this._data.resources().readBytes(readBytes);
      this._data.resources().writeBytes(writeBytes);
      return this;
    }

    /**
     * Appends the given ledger keys to the existing storage access footprint.
     * @param {xdr.LedgerKey[]} readOnly   read-only keys to add
     * @param {xdr.LedgerKey[]} readWrite  read-write keys to add
     * @returns {SorobanDataBuilder} this builder instance
     */
  }, {
    key: "appendFootprint",
    value: function appendFootprint(readOnly, readWrite) {
      return this.setFootprint(this.getReadOnly().concat(readOnly), this.getReadWrite().concat(readWrite));
    }

    /**
     * Sets the storage access footprint to be a certain set of ledger keys.
     *
     * You can also set each field explicitly via
     * {@link SorobanDataBuilder.setReadOnly} and
     * {@link SorobanDataBuilder.setReadWrite} or add to the existing footprint
     * via {@link SorobanDataBuilder.appendFootprint}.
     *
     * Passing `null|undefined` to either parameter will IGNORE the existing
     * values. If you want to clear them, pass `[]`, instead.
     *
     * @param {xdr.LedgerKey[]|null} [readOnly]   the set of ledger keys to set in
     *    the read-only portion of the transaction's `sorobanData`, or `null |
     *    undefined` to keep the existing keys
     * @param {xdr.LedgerKey[]|null} [readWrite]  the set of ledger keys to set in
     *    the read-write portion of the transaction's `sorobanData`, or `null |
     *    undefined` to keep the existing keys
     * @returns {SorobanDataBuilder} this builder instance
     */
  }, {
    key: "setFootprint",
    value: function setFootprint(readOnly, readWrite) {
      if (readOnly !== null) {
        // null means "leave me alone"
        this.setReadOnly(readOnly);
      }
      if (readWrite !== null) {
        this.setReadWrite(readWrite);
      }
      return this;
    }

    /**
     * @param {xdr.LedgerKey[]} readOnly  read-only keys in the access footprint
     * @returns {SorobanDataBuilder}
     */
  }, {
    key: "setReadOnly",
    value: function setReadOnly(readOnly) {
      this._data.resources().footprint().readOnly(readOnly !== null && readOnly !== void 0 ? readOnly : []);
      return this;
    }

    /**
     * @param {xdr.LedgerKey[]} readWrite  read-write keys in the access footprint
     * @returns {SorobanDataBuilder}
     */
  }, {
    key: "setReadWrite",
    value: function setReadWrite(readWrite) {
      this._data.resources().footprint().readWrite(readWrite !== null && readWrite !== void 0 ? readWrite : []);
      return this;
    }

    /**
     * @returns {xdr.SorobanTransactionData} a copy of the final data structure
     */
  }, {
    key: "build",
    value: function build() {
      return _xdr["default"].SorobanTransactionData.fromXDR(this._data.toXDR()); // clone
    }

    //
    // getters follow
    //

    /** @returns {xdr.LedgerKey[]} the read-only storage access pattern */
  }, {
    key: "getReadOnly",
    value: function getReadOnly() {
      return this.getFootprint().readOnly();
    }

    /** @returns {xdr.LedgerKey[]} the read-write storage access pattern */
  }, {
    key: "getReadWrite",
    value: function getReadWrite() {
      return this.getFootprint().readWrite();
    }

    /** @returns {xdr.LedgerFootprint} the storage access pattern */
  }, {
    key: "getFootprint",
    value: function getFootprint() {
      return this._data.resources().footprint();
    }
  }], [{
    key: "fromXDR",
    value: function fromXDR(data) {
      return _xdr["default"].SorobanTransactionData.fromXDR(data, typeof data === 'string' ? 'base64' : 'raw');
    }
  }]);
}();