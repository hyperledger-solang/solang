"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.buildChallengeTx = buildChallengeTx;
exports.gatherTxSigners = gatherTxSigners;
exports.readChallengeTx = readChallengeTx;
exports.verifyChallengeTxSigners = verifyChallengeTxSigners;
exports.verifyChallengeTxThreshold = verifyChallengeTxThreshold;
exports.verifyTxSignedBy = verifyTxSignedBy;
var _randombytes = _interopRequireDefault(require("randombytes"));
var _stellarBase = require("@stellar/stellar-base");
var _utils = require("../utils");
var _errors = require("./errors");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function _toConsumableArray(r) { return _arrayWithoutHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray(r) || _nonIterableSpread(); }
function _nonIterableSpread() { throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _arrayWithoutHoles(r) { if (Array.isArray(r)) return _arrayLikeToArray(r); }
function _createForOfIteratorHelper(r, e) { var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (!t) { if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e && r && "number" == typeof r.length) { t && (r = t); var _n = 0, F = function F() {}; return { s: F, n: function n() { return _n >= r.length ? { done: !0 } : { done: !1, value: r[_n++] }; }, e: function e(r) { throw r; }, f: F }; } throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); } var o, a = !0, u = !1; return { s: function s() { t = t.call(r); }, n: function n() { var r = t.next(); return a = r.done, r; }, e: function e(r) { u = !0, o = r; }, f: function f() { try { a || null == t.return || t.return(); } finally { if (u) throw o; } } }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _toArray(r) { return _arrayWithHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray(r) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArray(r) { if ("undefined" != typeof Symbol && null != r[Symbol.iterator] || null != r["@@iterator"]) return Array.from(r); }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function buildChallengeTx(serverKeypair, clientAccountID, homeDomain) {
  var timeout = arguments.length > 3 && arguments[3] !== undefined ? arguments[3] : 300;
  var networkPassphrase = arguments.length > 4 ? arguments[4] : undefined;
  var webAuthDomain = arguments.length > 5 ? arguments[5] : undefined;
  var memo = arguments.length > 6 && arguments[6] !== undefined ? arguments[6] : null;
  var clientDomain = arguments.length > 7 && arguments[7] !== undefined ? arguments[7] : null;
  var clientSigningKey = arguments.length > 8 && arguments[8] !== undefined ? arguments[8] : null;
  if (clientAccountID.startsWith("M") && memo) {
    throw Error("memo cannot be used if clientAccountID is a muxed account");
  }
  var account = new _stellarBase.Account(serverKeypair.publicKey(), "-1");
  var now = Math.floor(Date.now() / 1000);
  var value = (0, _randombytes.default)(48).toString("base64");
  var builder = new _stellarBase.TransactionBuilder(account, {
    fee: _stellarBase.BASE_FEE,
    networkPassphrase: networkPassphrase,
    timebounds: {
      minTime: now,
      maxTime: now + timeout
    }
  }).addOperation(_stellarBase.Operation.manageData({
    name: "".concat(homeDomain, " auth"),
    value: value,
    source: clientAccountID
  })).addOperation(_stellarBase.Operation.manageData({
    name: "web_auth_domain",
    value: webAuthDomain,
    source: account.accountId()
  }));
  if (clientDomain) {
    if (!clientSigningKey) {
      throw Error("clientSigningKey is required if clientDomain is provided");
    }
    builder.addOperation(_stellarBase.Operation.manageData({
      name: "client_domain",
      value: clientDomain,
      source: clientSigningKey
    }));
  }
  if (memo) {
    builder.addMemo(_stellarBase.Memo.id(memo));
  }
  var transaction = builder.build();
  transaction.sign(serverKeypair);
  return transaction.toEnvelope().toXDR("base64").toString();
}
function readChallengeTx(challengeTx, serverAccountID, networkPassphrase, homeDomains, webAuthDomain) {
  var _transaction$timeBoun;
  if (serverAccountID.startsWith("M")) {
    throw Error("Invalid serverAccountID: multiplexed accounts are not supported.");
  }
  var transaction;
  try {
    transaction = new _stellarBase.Transaction(challengeTx, networkPassphrase);
  } catch (_unused) {
    try {
      transaction = new _stellarBase.FeeBumpTransaction(challengeTx, networkPassphrase);
    } catch (_unused2) {
      throw new _errors.InvalidChallengeError("Invalid challenge: unable to deserialize challengeTx transaction string");
    }
    throw new _errors.InvalidChallengeError("Invalid challenge: expected a Transaction but received a FeeBumpTransaction");
  }
  var sequence = Number.parseInt(transaction.sequence, 10);
  if (sequence !== 0) {
    throw new _errors.InvalidChallengeError("The transaction sequence number should be zero");
  }
  if (transaction.source !== serverAccountID) {
    throw new _errors.InvalidChallengeError("The transaction source account is not equal to the server's account");
  }
  if (transaction.operations.length < 1) {
    throw new _errors.InvalidChallengeError("The transaction should contain at least one operation");
  }
  var _transaction$operatio = _toArray(transaction.operations),
    operation = _transaction$operatio[0],
    subsequentOperations = _transaction$operatio.slice(1);
  if (!operation.source) {
    throw new _errors.InvalidChallengeError("The transaction's operation should contain a source account");
  }
  var clientAccountID = operation.source;
  var memo = null;
  if (transaction.memo.type !== _stellarBase.MemoNone) {
    if (clientAccountID.startsWith("M")) {
      throw new _errors.InvalidChallengeError("The transaction has a memo but the client account ID is a muxed account");
    }
    if (transaction.memo.type !== _stellarBase.MemoID) {
      throw new _errors.InvalidChallengeError("The transaction's memo must be of type `id`");
    }
    memo = transaction.memo.value;
  }
  if (operation.type !== "manageData") {
    throw new _errors.InvalidChallengeError("The transaction's operation type should be 'manageData'");
  }
  if (transaction.timeBounds && Number.parseInt((_transaction$timeBoun = transaction.timeBounds) === null || _transaction$timeBoun === void 0 ? void 0 : _transaction$timeBoun.maxTime, 10) === _stellarBase.TimeoutInfinite) {
    throw new _errors.InvalidChallengeError("The transaction requires non-infinite timebounds");
  }
  if (!_utils.Utils.validateTimebounds(transaction, 60 * 5)) {
    throw new _errors.InvalidChallengeError("The transaction has expired");
  }
  if (operation.value === undefined) {
    throw new _errors.InvalidChallengeError("The transaction's operation values should not be null");
  }
  if (!operation.value) {
    throw new _errors.InvalidChallengeError("The transaction's operation value should not be null");
  }
  if (Buffer.from(operation.value.toString(), "base64").length !== 48) {
    throw new _errors.InvalidChallengeError("The transaction's operation value should be a 64 bytes base64 random string");
  }
  if (!homeDomains) {
    throw new _errors.InvalidChallengeError("Invalid homeDomains: a home domain must be provided for verification");
  }
  var matchedHomeDomain;
  if (typeof homeDomains === "string") {
    if ("".concat(homeDomains, " auth") === operation.name) {
      matchedHomeDomain = homeDomains;
    }
  } else if (Array.isArray(homeDomains)) {
    matchedHomeDomain = homeDomains.find(function (domain) {
      return "".concat(domain, " auth") === operation.name;
    });
  } else {
    throw new _errors.InvalidChallengeError("Invalid homeDomains: homeDomains type is ".concat(_typeof(homeDomains), " but should be a string or an array"));
  }
  if (!matchedHomeDomain) {
    throw new _errors.InvalidChallengeError("Invalid homeDomains: the transaction's operation key name does not match the expected home domain");
  }
  var _iterator = _createForOfIteratorHelper(subsequentOperations),
    _step;
  try {
    for (_iterator.s(); !(_step = _iterator.n()).done;) {
      var op = _step.value;
      if (op.type !== "manageData") {
        throw new _errors.InvalidChallengeError("The transaction has operations that are not of type 'manageData'");
      }
      if (op.source !== serverAccountID && op.name !== "client_domain") {
        throw new _errors.InvalidChallengeError("The transaction has operations that are unrecognized");
      }
      if (op.name === "web_auth_domain") {
        if (op.value === undefined) {
          throw new _errors.InvalidChallengeError("'web_auth_domain' operation value should not be null");
        }
        if (op.value.compare(Buffer.from(webAuthDomain))) {
          throw new _errors.InvalidChallengeError("'web_auth_domain' operation value does not match ".concat(webAuthDomain));
        }
      }
    }
  } catch (err) {
    _iterator.e(err);
  } finally {
    _iterator.f();
  }
  if (!verifyTxSignedBy(transaction, serverAccountID)) {
    throw new _errors.InvalidChallengeError("Transaction not signed by server: '".concat(serverAccountID, "'"));
  }
  return {
    tx: transaction,
    clientAccountID: clientAccountID,
    matchedHomeDomain: matchedHomeDomain,
    memo: memo
  };
}
function verifyChallengeTxThreshold(challengeTx, serverAccountID, networkPassphrase, threshold, signerSummary, homeDomains, webAuthDomain) {
  var signers = signerSummary.map(function (signer) {
    return signer.key;
  });
  var signersFound = verifyChallengeTxSigners(challengeTx, serverAccountID, networkPassphrase, signers, homeDomains, webAuthDomain);
  var weight = 0;
  var _loop = function _loop() {
    var _signerSummary$find;
    var signer = _signersFound[_i];
    var sigWeight = ((_signerSummary$find = signerSummary.find(function (s) {
      return s.key === signer;
    })) === null || _signerSummary$find === void 0 ? void 0 : _signerSummary$find.weight) || 0;
    weight += sigWeight;
  };
  for (var _i = 0, _signersFound = signersFound; _i < _signersFound.length; _i++) {
    _loop();
  }
  if (weight < threshold) {
    throw new _errors.InvalidChallengeError("signers with weight ".concat(weight, " do not meet threshold ").concat(threshold, "\""));
  }
  return signersFound;
}
function verifyChallengeTxSigners(challengeTx, serverAccountID, networkPassphrase, signers, homeDomains, webAuthDomain) {
  var _readChallengeTx = readChallengeTx(challengeTx, serverAccountID, networkPassphrase, homeDomains, webAuthDomain),
    tx = _readChallengeTx.tx;
  var serverKP;
  try {
    serverKP = _stellarBase.Keypair.fromPublicKey(serverAccountID);
  } catch (err) {
    throw new Error("Couldn't infer keypair from the provided 'serverAccountID': ".concat(err.message));
  }
  var clientSigners = new Set();
  var _iterator2 = _createForOfIteratorHelper(signers),
    _step2;
  try {
    for (_iterator2.s(); !(_step2 = _iterator2.n()).done;) {
      var _signer = _step2.value;
      if (_signer === serverKP.publicKey()) {
        continue;
      }
      if (_signer.charAt(0) !== "G") {
        continue;
      }
      clientSigners.add(_signer);
    }
  } catch (err) {
    _iterator2.e(err);
  } finally {
    _iterator2.f();
  }
  if (clientSigners.size === 0) {
    throw new _errors.InvalidChallengeError("No verifiable client signers provided, at least one G... address must be provided");
  }
  var clientSigningKey;
  var _iterator3 = _createForOfIteratorHelper(tx.operations),
    _step3;
  try {
    for (_iterator3.s(); !(_step3 = _iterator3.n()).done;) {
      var op = _step3.value;
      if (op.type === "manageData" && op.name === "client_domain") {
        if (clientSigningKey) {
          throw new _errors.InvalidChallengeError("Found more than one client_domain operation");
        }
        clientSigningKey = op.source;
      }
    }
  } catch (err) {
    _iterator3.e(err);
  } finally {
    _iterator3.f();
  }
  var allSigners = [serverKP.publicKey()].concat(_toConsumableArray(Array.from(clientSigners)));
  if (clientSigningKey) {
    allSigners.push(clientSigningKey);
  }
  var signersFound = gatherTxSigners(tx, allSigners);
  var serverSignatureFound = false;
  var clientSigningKeySignatureFound = false;
  for (var _i2 = 0, _signersFound2 = signersFound; _i2 < _signersFound2.length; _i2++) {
    var signer = _signersFound2[_i2];
    if (signer === serverKP.publicKey()) {
      serverSignatureFound = true;
    }
    if (signer === clientSigningKey) {
      clientSigningKeySignatureFound = true;
    }
  }
  if (!serverSignatureFound) {
    throw new _errors.InvalidChallengeError("Transaction not signed by server: '".concat(serverKP.publicKey(), "'"));
  }
  if (clientSigningKey && !clientSigningKeySignatureFound) {
    throw new _errors.InvalidChallengeError("Transaction not signed by the source account of the 'client_domain' " + "ManageData operation");
  }
  if (signersFound.length === 1) {
    throw new _errors.InvalidChallengeError("None of the given signers match the transaction signatures");
  }
  if (signersFound.length !== tx.signatures.length) {
    throw new _errors.InvalidChallengeError("Transaction has unrecognized signatures");
  }
  signersFound.splice(signersFound.indexOf(serverKP.publicKey()), 1);
  if (clientSigningKey) {
    signersFound.splice(signersFound.indexOf(clientSigningKey), 1);
  }
  return signersFound;
}
function verifyTxSignedBy(transaction, accountID) {
  return gatherTxSigners(transaction, [accountID]).length !== 0;
}
function gatherTxSigners(transaction, signers) {
  var hashedSignatureBase = transaction.hash();
  var txSignatures = _toConsumableArray(transaction.signatures);
  var signersFound = new Set();
  var _iterator4 = _createForOfIteratorHelper(signers),
    _step4;
  try {
    for (_iterator4.s(); !(_step4 = _iterator4.n()).done;) {
      var signer = _step4.value;
      if (txSignatures.length === 0) {
        break;
      }
      var keypair = void 0;
      try {
        keypair = _stellarBase.Keypair.fromPublicKey(signer);
      } catch (err) {
        throw new _errors.InvalidChallengeError("Signer is not a valid address: ".concat(err.message));
      }
      for (var i = 0; i < txSignatures.length; i++) {
        var decSig = txSignatures[i];
        if (!decSig.hint().equals(keypair.signatureHint())) {
          continue;
        }
        if (keypair.verify(hashedSignatureBase, decSig.signature())) {
          signersFound.add(signer);
          txSignatures.splice(i, 1);
          break;
        }
      }
    }
  } catch (err) {
    _iterator4.e(err);
  } finally {
    _iterator4.f();
  }
  return Array.from(signersFound);
}