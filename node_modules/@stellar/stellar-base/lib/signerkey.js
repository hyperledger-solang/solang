"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.SignerKey = void 0;
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
 * A container class with helpers to convert between signer keys
 * (`xdr.SignerKey`) and {@link StrKey}s.
 *
 * It's primarly used for manipulating the `extraSigners` precondition on a
 * {@link Transaction}.
 *
 * @see {@link TransactionBuilder.setExtraSigners}
 */
var SignerKey = exports.SignerKey = /*#__PURE__*/function () {
  function SignerKey() {
    _classCallCheck(this, SignerKey);
  }
  return _createClass(SignerKey, null, [{
    key: "decodeAddress",
    value:
    /**
     * Decodes a StrKey address into an xdr.SignerKey instance.
     *
     * Only ED25519 public keys (G...), pre-auth transactions (T...), hashes
     * (H...), and signed payloads (P...) can be signer keys.
     *
     * @param   {string} address  a StrKey-encoded signer address
     * @returns {xdr.SignerKey}
     */
    function decodeAddress(address) {
      var signerKeyMap = {
        ed25519PublicKey: _xdr["default"].SignerKey.signerKeyTypeEd25519,
        preAuthTx: _xdr["default"].SignerKey.signerKeyTypePreAuthTx,
        sha256Hash: _xdr["default"].SignerKey.signerKeyTypeHashX,
        signedPayload: _xdr["default"].SignerKey.signerKeyTypeEd25519SignedPayload
      };
      var vb = _strkey.StrKey.getVersionByteForPrefix(address);
      var encoder = signerKeyMap[vb];
      if (!encoder) {
        throw new Error("invalid signer key type (".concat(vb, ")"));
      }
      var raw = (0, _strkey.decodeCheck)(vb, address);
      switch (vb) {
        case 'signedPayload':
          return encoder(new _xdr["default"].SignerKeyEd25519SignedPayload({
            ed25519: raw.slice(0, 32),
            payload: raw.slice(32 + 4)
          }));
        case 'ed25519PublicKey': // falls through
        case 'preAuthTx': // falls through
        case 'sha256Hash': // falls through
        default:
          return encoder(raw);
      }
    }

    /**
     * Encodes a signer key into its StrKey equivalent.
     *
     * @param   {xdr.SignerKey} signerKey   the signer
     * @returns {string} the StrKey representation of the signer
     */
  }, {
    key: "encodeSignerKey",
    value: function encodeSignerKey(signerKey) {
      var strkeyType;
      var raw;
      switch (signerKey["switch"]()) {
        case _xdr["default"].SignerKeyType.signerKeyTypeEd25519():
          strkeyType = 'ed25519PublicKey';
          raw = signerKey.value();
          break;
        case _xdr["default"].SignerKeyType.signerKeyTypePreAuthTx():
          strkeyType = 'preAuthTx';
          raw = signerKey.value();
          break;
        case _xdr["default"].SignerKeyType.signerKeyTypeHashX():
          strkeyType = 'sha256Hash';
          raw = signerKey.value();
          break;
        case _xdr["default"].SignerKeyType.signerKeyTypeEd25519SignedPayload():
          strkeyType = 'signedPayload';
          raw = signerKey.ed25519SignedPayload().toXDR('raw');
          break;
        default:
          throw new Error("invalid SignerKey (type: ".concat(signerKey["switch"](), ")"));
      }
      return (0, _strkey.encodeCheck)(strkeyType, raw);
    }
  }]);
}();