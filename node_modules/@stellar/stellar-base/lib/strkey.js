"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.StrKey = void 0;
exports.decodeCheck = decodeCheck;
exports.encodeCheck = encodeCheck;
var _base = _interopRequireDefault(require("base32.js"));
var _checksum = require("./util/checksum");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); } /* eslint no-bitwise: ["error", {"allow": ["<<", ">>", "^", "&", "&="]}] */
var versionBytes = {
  ed25519PublicKey: 6 << 3,
  // G (when encoded in base32)
  ed25519SecretSeed: 18 << 3,
  // S
  med25519PublicKey: 12 << 3,
  // M
  preAuthTx: 19 << 3,
  // T
  sha256Hash: 23 << 3,
  // X
  signedPayload: 15 << 3,
  // P
  contract: 2 << 3 // C
};
var strkeyTypes = {
  G: 'ed25519PublicKey',
  S: 'ed25519SecretSeed',
  M: 'med25519PublicKey',
  T: 'preAuthTx',
  X: 'sha256Hash',
  P: 'signedPayload',
  C: 'contract'
};

/**
 * StrKey is a helper class that allows encoding and decoding Stellar keys
 * to/from strings, i.e. between their binary (Buffer, xdr.PublicKey, etc.) and
 * string (i.e. "GABCD...", etc.) representations.
 */
var StrKey = exports.StrKey = /*#__PURE__*/function () {
  function StrKey() {
    _classCallCheck(this, StrKey);
  }
  return _createClass(StrKey, null, [{
    key: "encodeEd25519PublicKey",
    value:
    /**
     * Encodes `data` to strkey ed25519 public key.
     *
     * @param   {Buffer} data   raw data to encode
     * @returns {string}        "G..." representation of the key
     */
    function encodeEd25519PublicKey(data) {
      return encodeCheck('ed25519PublicKey', data);
    }

    /**
     * Decodes strkey ed25519 public key to raw data.
     *
     * If the parameter is a muxed account key ("M..."), this will only encode it
     * as a basic Ed25519 key (as if in "G..." format).
     *
     * @param   {string} data   "G..." (or "M...") key representation to decode
     * @returns {Buffer}        raw key
     */
  }, {
    key: "decodeEd25519PublicKey",
    value: function decodeEd25519PublicKey(data) {
      return decodeCheck('ed25519PublicKey', data);
    }

    /**
     * Returns true if the given Stellar public key is a valid ed25519 public key.
     * @param {string} publicKey public key to check
     * @returns {boolean}
     */
  }, {
    key: "isValidEd25519PublicKey",
    value: function isValidEd25519PublicKey(publicKey) {
      return isValid('ed25519PublicKey', publicKey);
    }

    /**
     * Encodes data to strkey ed25519 seed.
     * @param {Buffer} data data to encode
     * @returns {string}
     */
  }, {
    key: "encodeEd25519SecretSeed",
    value: function encodeEd25519SecretSeed(data) {
      return encodeCheck('ed25519SecretSeed', data);
    }

    /**
     * Decodes strkey ed25519 seed to raw data.
     * @param {string} address data to decode
     * @returns {Buffer}
     */
  }, {
    key: "decodeEd25519SecretSeed",
    value: function decodeEd25519SecretSeed(address) {
      return decodeCheck('ed25519SecretSeed', address);
    }

    /**
     * Returns true if the given Stellar secret key is a valid ed25519 secret seed.
     * @param {string} seed seed to check
     * @returns {boolean}
     */
  }, {
    key: "isValidEd25519SecretSeed",
    value: function isValidEd25519SecretSeed(seed) {
      return isValid('ed25519SecretSeed', seed);
    }

    /**
     * Encodes data to strkey med25519 public key.
     * @param {Buffer} data data to encode
     * @returns {string}
     */
  }, {
    key: "encodeMed25519PublicKey",
    value: function encodeMed25519PublicKey(data) {
      return encodeCheck('med25519PublicKey', data);
    }

    /**
     * Decodes strkey med25519 public key to raw data.
     * @param {string} address data to decode
     * @returns {Buffer}
     */
  }, {
    key: "decodeMed25519PublicKey",
    value: function decodeMed25519PublicKey(address) {
      return decodeCheck('med25519PublicKey', address);
    }

    /**
     * Returns true if the given Stellar public key is a valid med25519 public key.
     * @param {string} publicKey public key to check
     * @returns {boolean}
     */
  }, {
    key: "isValidMed25519PublicKey",
    value: function isValidMed25519PublicKey(publicKey) {
      return isValid('med25519PublicKey', publicKey);
    }

    /**
     * Encodes data to strkey preAuthTx.
     * @param {Buffer} data data to encode
     * @returns {string}
     */
  }, {
    key: "encodePreAuthTx",
    value: function encodePreAuthTx(data) {
      return encodeCheck('preAuthTx', data);
    }

    /**
     * Decodes strkey PreAuthTx to raw data.
     * @param {string} address data to decode
     * @returns {Buffer}
     */
  }, {
    key: "decodePreAuthTx",
    value: function decodePreAuthTx(address) {
      return decodeCheck('preAuthTx', address);
    }

    /**
     * Encodes data to strkey sha256 hash.
     * @param {Buffer} data data to encode
     * @returns {string}
     */
  }, {
    key: "encodeSha256Hash",
    value: function encodeSha256Hash(data) {
      return encodeCheck('sha256Hash', data);
    }

    /**
     * Decodes strkey sha256 hash to raw data.
     * @param {string} address data to decode
     * @returns {Buffer}
     */
  }, {
    key: "decodeSha256Hash",
    value: function decodeSha256Hash(address) {
      return decodeCheck('sha256Hash', address);
    }

    /**
     * Encodes raw data to strkey signed payload (P...).
     * @param   {Buffer} data  data to encode
     * @returns {string}
     */
  }, {
    key: "encodeSignedPayload",
    value: function encodeSignedPayload(data) {
      return encodeCheck('signedPayload', data);
    }

    /**
     * Decodes strkey signed payload (P...) to raw data.
     * @param   {string} address  address to decode
     * @returns {Buffer}
     */
  }, {
    key: "decodeSignedPayload",
    value: function decodeSignedPayload(address) {
      return decodeCheck('signedPayload', address);
    }

    /**
     * Checks validity of alleged signed payload (P...) strkey address.
     * @param   {string} address  signer key to check
     * @returns {boolean}
     */
  }, {
    key: "isValidSignedPayload",
    value: function isValidSignedPayload(address) {
      return isValid('signedPayload', address);
    }

    /**
     * Encodes raw data to strkey contract (C...).
     * @param   {Buffer} data  data to encode
     * @returns {string}
     */
  }, {
    key: "encodeContract",
    value: function encodeContract(data) {
      return encodeCheck('contract', data);
    }

    /**
     * Decodes strkey contract (C...) to raw data.
     * @param   {string} address  address to decode
     * @returns {Buffer}
     */
  }, {
    key: "decodeContract",
    value: function decodeContract(address) {
      return decodeCheck('contract', address);
    }

    /**
     * Checks validity of alleged contract (C...) strkey address.
     * @param   {string} address  signer key to check
     * @returns {boolean}
     */
  }, {
    key: "isValidContract",
    value: function isValidContract(address) {
      return isValid('contract', address);
    }
  }, {
    key: "getVersionByteForPrefix",
    value: function getVersionByteForPrefix(address) {
      return strkeyTypes[address[0]];
    }
  }]);
}();
/**
 * Sanity-checks whether or not a strkey *appears* valid.
 *
 * @param  {string}  versionByteName the type of strkey to expect in `encoded`
 * @param  {string}  encoded         the strkey to validate
 *
 * @return {Boolean} whether or not the `encoded` strkey appears valid for the
 *     `versionByteName` strkey type (see `versionBytes`, above).
 *
 * @note This isn't a *definitive* check of validity, but rather a best-effort
 *     check based on (a) input length, (b) whether or not it can be decoded,
 *     and (c) output length.
 */
function isValid(versionByteName, encoded) {
  if (typeof encoded !== 'string') {
    return false;
  }

  // basic length checks on the strkey lengths
  switch (versionByteName) {
    case 'ed25519PublicKey': // falls through
    case 'ed25519SecretSeed': // falls through
    case 'preAuthTx': // falls through
    case 'sha256Hash': // falls through
    case 'contract':
      if (encoded.length !== 56) {
        return false;
      }
      break;
    case 'med25519PublicKey':
      if (encoded.length !== 69) {
        return false;
      }
      break;
    case 'signedPayload':
      if (encoded.length < 56 || encoded.length > 165) {
        return false;
      }
      break;
    default:
      return false;
  }
  var decoded = '';
  try {
    decoded = decodeCheck(versionByteName, encoded);
  } catch (err) {
    return false;
  }

  // basic length checks on the resulting buffer sizes
  switch (versionByteName) {
    case 'ed25519PublicKey': // falls through
    case 'ed25519SecretSeed': // falls through
    case 'preAuthTx': // falls through
    case 'sha256Hash': // falls through
    case 'contract':
      return decoded.length === 32;
    case 'med25519PublicKey':
      return decoded.length === 40;
    // +8 bytes for the ID

    case 'signedPayload':
      return (
        // 32 for the signer, +4 for the payload size, then either +4 for the
        // min or +64 for the max payload
        decoded.length >= 32 + 4 + 4 && decoded.length <= 32 + 4 + 64
      );
    default:
      return false;
  }
}
function decodeCheck(versionByteName, encoded) {
  if (typeof encoded !== 'string') {
    throw new TypeError('encoded argument must be of type String');
  }
  var decoded = _base["default"].decode(encoded);
  var versionByte = decoded[0];
  var payload = decoded.slice(0, -2);
  var data = payload.slice(1);
  var checksum = decoded.slice(-2);
  if (encoded !== _base["default"].encode(decoded)) {
    throw new Error('invalid encoded string');
  }
  var expectedVersion = versionBytes[versionByteName];
  if (expectedVersion === undefined) {
    throw new Error("".concat(versionByteName, " is not a valid version byte name. ") + "Expected one of ".concat(Object.keys(versionBytes).join(', ')));
  }
  if (versionByte !== expectedVersion) {
    throw new Error("invalid version byte. expected ".concat(expectedVersion, ", got ").concat(versionByte));
  }
  var expectedChecksum = calculateChecksum(payload);
  if (!(0, _checksum.verifyChecksum)(expectedChecksum, checksum)) {
    throw new Error("invalid checksum");
  }
  return Buffer.from(data);
}
function encodeCheck(versionByteName, data) {
  if (data === null || data === undefined) {
    throw new Error('cannot encode null data');
  }
  var versionByte = versionBytes[versionByteName];
  if (versionByte === undefined) {
    throw new Error("".concat(versionByteName, " is not a valid version byte name. ") + "Expected one of ".concat(Object.keys(versionBytes).join(', ')));
  }
  data = Buffer.from(data);
  var versionBuffer = Buffer.from([versionByte]);
  var payload = Buffer.concat([versionBuffer, data]);
  var checksum = Buffer.from(calculateChecksum(payload));
  var unencoded = Buffer.concat([payload, checksum]);
  return _base["default"].encode(unencoded);
}

// Computes the CRC16-XModem checksum of `payload` in little-endian order
function calculateChecksum(payload) {
  var crcTable = [0x0000, 0x1021, 0x2042, 0x3063, 0x4084, 0x50a5, 0x60c6, 0x70e7, 0x8108, 0x9129, 0xa14a, 0xb16b, 0xc18c, 0xd1ad, 0xe1ce, 0xf1ef, 0x1231, 0x0210, 0x3273, 0x2252, 0x52b5, 0x4294, 0x72f7, 0x62d6, 0x9339, 0x8318, 0xb37b, 0xa35a, 0xd3bd, 0xc39c, 0xf3ff, 0xe3de, 0x2462, 0x3443, 0x0420, 0x1401, 0x64e6, 0x74c7, 0x44a4, 0x5485, 0xa56a, 0xb54b, 0x8528, 0x9509, 0xe5ee, 0xf5cf, 0xc5ac, 0xd58d, 0x3653, 0x2672, 0x1611, 0x0630, 0x76d7, 0x66f6, 0x5695, 0x46b4, 0xb75b, 0xa77a, 0x9719, 0x8738, 0xf7df, 0xe7fe, 0xd79d, 0xc7bc, 0x48c4, 0x58e5, 0x6886, 0x78a7, 0x0840, 0x1861, 0x2802, 0x3823, 0xc9cc, 0xd9ed, 0xe98e, 0xf9af, 0x8948, 0x9969, 0xa90a, 0xb92b, 0x5af5, 0x4ad4, 0x7ab7, 0x6a96, 0x1a71, 0x0a50, 0x3a33, 0x2a12, 0xdbfd, 0xcbdc, 0xfbbf, 0xeb9e, 0x9b79, 0x8b58, 0xbb3b, 0xab1a, 0x6ca6, 0x7c87, 0x4ce4, 0x5cc5, 0x2c22, 0x3c03, 0x0c60, 0x1c41, 0xedae, 0xfd8f, 0xcdec, 0xddcd, 0xad2a, 0xbd0b, 0x8d68, 0x9d49, 0x7e97, 0x6eb6, 0x5ed5, 0x4ef4, 0x3e13, 0x2e32, 0x1e51, 0x0e70, 0xff9f, 0xefbe, 0xdfdd, 0xcffc, 0xbf1b, 0xaf3a, 0x9f59, 0x8f78, 0x9188, 0x81a9, 0xb1ca, 0xa1eb, 0xd10c, 0xc12d, 0xf14e, 0xe16f, 0x1080, 0x00a1, 0x30c2, 0x20e3, 0x5004, 0x4025, 0x7046, 0x6067, 0x83b9, 0x9398, 0xa3fb, 0xb3da, 0xc33d, 0xd31c, 0xe37f, 0xf35e, 0x02b1, 0x1290, 0x22f3, 0x32d2, 0x4235, 0x5214, 0x6277, 0x7256, 0xb5ea, 0xa5cb, 0x95a8, 0x8589, 0xf56e, 0xe54f, 0xd52c, 0xc50d, 0x34e2, 0x24c3, 0x14a0, 0x0481, 0x7466, 0x6447, 0x5424, 0x4405, 0xa7db, 0xb7fa, 0x8799, 0x97b8, 0xe75f, 0xf77e, 0xc71d, 0xd73c, 0x26d3, 0x36f2, 0x0691, 0x16b0, 0x6657, 0x7676, 0x4615, 0x5634, 0xd94c, 0xc96d, 0xf90e, 0xe92f, 0x99c8, 0x89e9, 0xb98a, 0xa9ab, 0x5844, 0x4865, 0x7806, 0x6827, 0x18c0, 0x08e1, 0x3882, 0x28a3, 0xcb7d, 0xdb5c, 0xeb3f, 0xfb1e, 0x8bf9, 0x9bd8, 0xabbb, 0xbb9a, 0x4a75, 0x5a54, 0x6a37, 0x7a16, 0x0af1, 0x1ad0, 0x2ab3, 0x3a92, 0xfd2e, 0xed0f, 0xdd6c, 0xcd4d, 0xbdaa, 0xad8b, 0x9de8, 0x8dc9, 0x7c26, 0x6c07, 0x5c64, 0x4c45, 0x3ca2, 0x2c83, 0x1ce0, 0x0cc1, 0xef1f, 0xff3e, 0xcf5d, 0xdf7c, 0xaf9b, 0xbfba, 0x8fd9, 0x9ff8, 0x6e17, 0x7e36, 0x4e55, 0x5e74, 0x2e93, 0x3eb2, 0x0ed1, 0x1ef0];
  var crc16 = 0x0;
  for (var i = 0; i < payload.length; i += 1) {
    var _byte = payload[i];
    var lookupIndex = crc16 >> 8 ^ _byte;
    crc16 = crc16 << 8 ^ crcTable[lookupIndex];
    crc16 &= 0xffff;
  }
  var checksum = new Uint8Array(2);
  checksum[0] = crc16 & 0xff;
  checksum[1] = crc16 >> 8 & 0xff;
  return checksum;
}