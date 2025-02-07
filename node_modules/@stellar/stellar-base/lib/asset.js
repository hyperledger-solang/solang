"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Asset = void 0;
var _util = require("./util/util");
var _xdr = _interopRequireDefault(require("./xdr"));
var _keypair = require("./keypair");
var _strkey = require("./strkey");
var _hashing = require("./hashing");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * Asset class represents an asset, either the native asset (`XLM`)
 * or an asset code / issuer account ID pair.
 *
 * An asset code describes an asset code and issuer pair. In the case of the native
 * asset XLM, the issuer will be null.
 *
 * @constructor
 * @param {string} code - The asset code.
 * @param {string} issuer - The account ID of the issuer.
 */
var Asset = exports.Asset = /*#__PURE__*/function () {
  function Asset(code, issuer) {
    _classCallCheck(this, Asset);
    if (!/^[a-zA-Z0-9]{1,12}$/.test(code)) {
      throw new Error('Asset code is invalid (maximum alphanumeric, 12 characters at max)');
    }
    if (String(code).toLowerCase() !== 'xlm' && !issuer) {
      throw new Error('Issuer cannot be null');
    }
    if (issuer && !_strkey.StrKey.isValidEd25519PublicKey(issuer)) {
      throw new Error('Issuer is invalid');
    }
    if (String(code).toLowerCase() === 'xlm') {
      // transform all xLM, Xlm, etc. variants -> XLM
      this.code = 'XLM';
    } else {
      this.code = code;
    }
    this.issuer = issuer;
  }

  /**
   * Returns an asset object for the native asset.
   * @Return {Asset}
   */
  return _createClass(Asset, [{
    key: "toXDRObject",
    value:
    /**
     * Returns the xdr.Asset object for this asset.
     * @returns {xdr.Asset} XDR asset object
     */
    function toXDRObject() {
      return this._toXDRObject(_xdr["default"].Asset);
    }

    /**
     * Returns the xdr.ChangeTrustAsset object for this asset.
     * @returns {xdr.ChangeTrustAsset} XDR asset object
     */
  }, {
    key: "toChangeTrustXDRObject",
    value: function toChangeTrustXDRObject() {
      return this._toXDRObject(_xdr["default"].ChangeTrustAsset);
    }

    /**
     * Returns the xdr.TrustLineAsset object for this asset.
     * @returns {xdr.TrustLineAsset} XDR asset object
     */
  }, {
    key: "toTrustLineXDRObject",
    value: function toTrustLineXDRObject() {
      return this._toXDRObject(_xdr["default"].TrustLineAsset);
    }

    /**
     * Returns the would-be contract ID (`C...` format) for this asset on a given
     * network.
     *
     * @param {string}    networkPassphrase   indicates which network the contract
     *    ID should refer to, since every network will have a unique ID for the
     *    same contract (see {@link Networks} for options)
     *
     * @returns {string}  the strkey-encoded (`C...`) contract ID for this asset
     *
     * @warning This makes no guarantee that this contract actually *exists*.
     */
  }, {
    key: "contractId",
    value: function contractId(networkPassphrase) {
      var networkId = (0, _hashing.hash)(Buffer.from(networkPassphrase));
      var preimage = _xdr["default"].HashIdPreimage.envelopeTypeContractId(new _xdr["default"].HashIdPreimageContractId({
        networkId: networkId,
        contractIdPreimage: _xdr["default"].ContractIdPreimage.contractIdPreimageFromAsset(this.toXDRObject())
      }));
      return _strkey.StrKey.encodeContract((0, _hashing.hash)(preimage.toXDR()));
    }

    /**
     * Returns the xdr object for this asset.
     * @param {xdr.Asset | xdr.ChangeTrustAsset} xdrAsset - The asset xdr object.
     * @returns {xdr.Asset | xdr.ChangeTrustAsset | xdr.TrustLineAsset} XDR Asset object
     */
  }, {
    key: "_toXDRObject",
    value: function _toXDRObject() {
      var xdrAsset = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : _xdr["default"].Asset;
      if (this.isNative()) {
        return xdrAsset.assetTypeNative();
      }
      var xdrType;
      var xdrTypeString;
      if (this.code.length <= 4) {
        xdrType = _xdr["default"].AlphaNum4;
        xdrTypeString = 'assetTypeCreditAlphanum4';
      } else {
        xdrType = _xdr["default"].AlphaNum12;
        xdrTypeString = 'assetTypeCreditAlphanum12';
      }

      // pad code with null bytes if necessary
      var padLength = this.code.length <= 4 ? 4 : 12;
      var paddedCode = this.code.padEnd(padLength, '\0');

      // eslint-disable-next-line new-cap
      var assetType = new xdrType({
        assetCode: paddedCode,
        issuer: _keypair.Keypair.fromPublicKey(this.issuer).xdrAccountId()
      });
      return new xdrAsset(xdrTypeString, assetType);
    }

    /**
     * @returns {string} Asset code
     */
  }, {
    key: "getCode",
    value: function getCode() {
      if (this.code === undefined) {
        return undefined;
      }
      return String(this.code);
    }

    /**
     * @returns {string} Asset issuer
     */
  }, {
    key: "getIssuer",
    value: function getIssuer() {
      if (this.issuer === undefined) {
        return undefined;
      }
      return String(this.issuer);
    }

    /**
     * @see [Assets concept](https://developers.stellar.org/docs/glossary/assets/)
     * @returns {string} Asset type. Can be one of following types:
     *
     *  - `native`,
     *  - `credit_alphanum4`,
     *  - `credit_alphanum12`, or
     *  - `unknown` as the error case (which should never occur)
     */
  }, {
    key: "getAssetType",
    value: function getAssetType() {
      switch (this.getRawAssetType().value) {
        case _xdr["default"].AssetType.assetTypeNative().value:
          return 'native';
        case _xdr["default"].AssetType.assetTypeCreditAlphanum4().value:
          return 'credit_alphanum4';
        case _xdr["default"].AssetType.assetTypeCreditAlphanum12().value:
          return 'credit_alphanum12';
        default:
          return 'unknown';
      }
    }

    /**
     * @returns {xdr.AssetType}  the raw XDR representation of the asset type
     */
  }, {
    key: "getRawAssetType",
    value: function getRawAssetType() {
      if (this.isNative()) {
        return _xdr["default"].AssetType.assetTypeNative();
      }
      if (this.code.length <= 4) {
        return _xdr["default"].AssetType.assetTypeCreditAlphanum4();
      }
      return _xdr["default"].AssetType.assetTypeCreditAlphanum12();
    }

    /**
     * @returns {boolean}  true if this asset object is the native asset.
     */
  }, {
    key: "isNative",
    value: function isNative() {
      return !this.issuer;
    }

    /**
     * @param {Asset} asset Asset to compare
     * @returns {boolean} true if this asset equals the given asset.
     */
  }, {
    key: "equals",
    value: function equals(asset) {
      return this.code === asset.getCode() && this.issuer === asset.getIssuer();
    }
  }, {
    key: "toString",
    value: function toString() {
      if (this.isNative()) {
        return 'native';
      }
      return "".concat(this.getCode(), ":").concat(this.getIssuer());
    }

    /**
     * Compares two assets according to the criteria:
     *
     *  1. First compare the type (native < alphanum4 < alphanum12).
     *  2. If the types are equal, compare the assets codes.
     *  3. If the asset codes are equal, compare the issuers.
     *
     * @param   {Asset} assetA - the first asset
     * @param   {Asset} assetB - the second asset
     * @returns {number} `-1` if assetA < assetB, `0` if assetA == assetB, `1` if assetA > assetB.
     *
     * @static
     * @memberof Asset
     */
  }], [{
    key: "native",
    value: function _native() {
      return new Asset('XLM');
    }

    /**
     * Returns an asset object from its XDR object representation.
     * @param {xdr.Asset} assetXdr - The asset xdr object.
     * @returns {Asset}
     */
  }, {
    key: "fromOperation",
    value: function fromOperation(assetXdr) {
      var anum;
      var code;
      var issuer;
      switch (assetXdr["switch"]()) {
        case _xdr["default"].AssetType.assetTypeNative():
          return this["native"]();
        case _xdr["default"].AssetType.assetTypeCreditAlphanum4():
          anum = assetXdr.alphaNum4();
        /* falls through */
        case _xdr["default"].AssetType.assetTypeCreditAlphanum12():
          anum = anum || assetXdr.alphaNum12();
          issuer = _strkey.StrKey.encodeEd25519PublicKey(anum.issuer().ed25519());
          code = (0, _util.trimEnd)(anum.assetCode(), '\0');
          return new this(code, issuer);
        default:
          throw new Error("Invalid asset type: ".concat(assetXdr["switch"]().name));
      }
    }
  }, {
    key: "compare",
    value: function compare(assetA, assetB) {
      if (!assetA || !(assetA instanceof Asset)) {
        throw new Error('assetA is invalid');
      }
      if (!assetB || !(assetB instanceof Asset)) {
        throw new Error('assetB is invalid');
      }
      if (assetA.equals(assetB)) {
        return 0;
      }

      // Compare asset types.
      var xdrAtype = assetA.getRawAssetType().value;
      var xdrBtype = assetB.getRawAssetType().value;
      if (xdrAtype !== xdrBtype) {
        return xdrAtype < xdrBtype ? -1 : 1;
      }

      // Compare asset codes.
      var result = asciiCompare(assetA.getCode(), assetB.getCode());
      if (result !== 0) {
        return result;
      }

      // Compare asset issuers.
      return asciiCompare(assetA.getIssuer(), assetB.getIssuer());
    }
  }]);
}();
/**
 * Compares two ASCII strings in lexographic order with uppercase precedence.
 *
 * @param   {string} a - the first string to compare
 * @param   {string} b - the second
 * @returns {number} like all `compare()`s:
 *     -1 if `a < b`, 0 if `a == b`, and 1 if `a > b`
 *
 * @warning No type-checks are done on the parameters
 */
function asciiCompare(a, b) {
  return Buffer.compare(Buffer.from(a, 'ascii'), Buffer.from(b, 'ascii'));
}