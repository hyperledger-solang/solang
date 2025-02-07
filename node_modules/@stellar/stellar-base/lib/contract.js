"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Contract = void 0;
var _address = require("./address");
var _operation = require("./operation");
var _xdr = _interopRequireDefault(require("./xdr"));
var _strkey = require("./strkey");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * Create a new Contract object.
 *
 * `Contract` represents a single contract in the Stellar network, embodying the
 * interface of the contract. See
 * [Contracts](https://soroban.stellar.org/docs/learn/interacting-with-contracts)
 * for more information about how contracts work in Stellar.
 *
 * @constructor
 *
 * @param {string} contractId - ID of the contract (ex.
 *     `CA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAXE`).
 */
var Contract = exports.Contract = /*#__PURE__*/function () {
  function Contract(contractId) {
    _classCallCheck(this, Contract);
    try {
      // First, try it as a strkey
      this._id = _strkey.StrKey.decodeContract(contractId);
    } catch (_) {
      throw new Error("Invalid contract ID: ".concat(contractId));
    }
  }

  /**
   * Returns Stellar contract ID as a strkey, ex.
   * `CA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAXE`.
   * @returns {string}
   */
  return _createClass(Contract, [{
    key: "contractId",
    value: function contractId() {
      return _strkey.StrKey.encodeContract(this._id);
    }

    /** @returns {string} the ID as a strkey (C...) */
  }, {
    key: "toString",
    value: function toString() {
      return this.contractId();
    }

    /** @returns {Address} the wrapped address of this contract */
  }, {
    key: "address",
    value: function address() {
      return _address.Address.contract(this._id);
    }

    /**
     * Returns an operation that will invoke this contract call.
     *
     * @param {string}        method   name of the method to call
     * @param {...xdr.ScVal}  params   arguments to pass to the function call
     *
     * @returns {xdr.Operation}   an InvokeHostFunctionOp operation to call the
     *    contract with the given method and parameters
     *
     * @see Operation.invokeHostFunction
     * @see Operation.invokeContractFunction
     * @see Operation.createCustomContract
     * @see Operation.createStellarAssetContract
     * @see Operation.uploadContractWasm
     */
  }, {
    key: "call",
    value: function call(method) {
      for (var _len = arguments.length, params = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++) {
        params[_key - 1] = arguments[_key];
      }
      return _operation.Operation.invokeContractFunction({
        contract: this.address().toString(),
        "function": method,
        args: params
      });
    }

    /**
     * Returns the read-only footprint entries necessary for any invocations to
     * this contract, for convenience when manually adding it to your
     * transaction's overall footprint or doing bump/restore operations.
     *
     * @returns {xdr.LedgerKey} the ledger key for the deployed contract instance
     */
  }, {
    key: "getFootprint",
    value: function getFootprint() {
      return _xdr["default"].LedgerKey.contractData(new _xdr["default"].LedgerKeyContractData({
        contract: this.address().toScAddress(),
        key: _xdr["default"].ScVal.scvLedgerKeyContractInstance(),
        durability: _xdr["default"].ContractDataDurability.persistent()
      }));
    }
  }]);
}();