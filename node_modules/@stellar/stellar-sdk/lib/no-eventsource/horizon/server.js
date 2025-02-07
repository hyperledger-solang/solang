"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.SUBMIT_TRANSACTION_TIMEOUT = exports.HorizonServer = void 0;
var _bignumber = _interopRequireDefault(require("bignumber.js"));
var _stellarBase = require("@stellar/stellar-base");
var _urijs = _interopRequireDefault(require("urijs"));
var _call_builder = require("./call_builder");
var _config = require("../config");
var _errors = require("../errors");
var _account_call_builder = require("./account_call_builder");
var _account_response = require("./account_response");
var _assets_call_builder = require("./assets_call_builder");
var _claimable_balances_call_builder = require("./claimable_balances_call_builder");
var _effect_call_builder = require("./effect_call_builder");
var _friendbot_builder = require("./friendbot_builder");
var _ledger_call_builder = require("./ledger_call_builder");
var _liquidity_pool_call_builder = require("./liquidity_pool_call_builder");
var _offer_call_builder = require("./offer_call_builder");
var _operation_call_builder = require("./operation_call_builder");
var _orderbook_call_builder = require("./orderbook_call_builder");
var _payment_call_builder = require("./payment_call_builder");
var _strict_receive_path_call_builder = require("./strict_receive_path_call_builder");
var _strict_send_path_call_builder = require("./strict_send_path_call_builder");
var _trade_aggregation_call_builder = require("./trade_aggregation_call_builder");
var _trades_call_builder = require("./trades_call_builder");
var _transaction_call_builder = require("./transaction_call_builder");
var _horizon_axios_client = _interopRequireWildcard(require("./horizon_axios_client"));
function _getRequireWildcardCache(e) { if ("function" != typeof WeakMap) return null; var r = new WeakMap(), t = new WeakMap(); return (_getRequireWildcardCache = function _getRequireWildcardCache(e) { return e ? t : r; })(e); }
function _interopRequireWildcard(e, r) { if (!r && e && e.__esModule) return e; if (null === e || "object" != _typeof(e) && "function" != typeof e) return { default: e }; var t = _getRequireWildcardCache(r); if (t && t.has(e)) return t.get(e); var n = { __proto__: null }, a = Object.defineProperty && Object.getOwnPropertyDescriptor; for (var u in e) if ("default" !== u && {}.hasOwnProperty.call(e, u)) { var i = a ? Object.getOwnPropertyDescriptor(e, u) : null; i && (i.get || i.set) ? Object.defineProperty(n, u, i) : n[u] = e[u]; } return n.default = e, t && t.set(e, n), n; }
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _regeneratorRuntime() { "use strict"; _regeneratorRuntime = function _regeneratorRuntime() { return e; }; var t, e = {}, r = Object.prototype, n = r.hasOwnProperty, o = Object.defineProperty || function (t, e, r) { t[e] = r.value; }, i = "function" == typeof Symbol ? Symbol : {}, a = i.iterator || "@@iterator", c = i.asyncIterator || "@@asyncIterator", u = i.toStringTag || "@@toStringTag"; function define(t, e, r) { return Object.defineProperty(t, e, { value: r, enumerable: !0, configurable: !0, writable: !0 }), t[e]; } try { define({}, ""); } catch (t) { define = function define(t, e, r) { return t[e] = r; }; } function wrap(t, e, r, n) { var i = e && e.prototype instanceof Generator ? e : Generator, a = Object.create(i.prototype), c = new Context(n || []); return o(a, "_invoke", { value: makeInvokeMethod(t, r, c) }), a; } function tryCatch(t, e, r) { try { return { type: "normal", arg: t.call(e, r) }; } catch (t) { return { type: "throw", arg: t }; } } e.wrap = wrap; var h = "suspendedStart", l = "suspendedYield", f = "executing", s = "completed", y = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} var p = {}; define(p, a, function () { return this; }); var d = Object.getPrototypeOf, v = d && d(d(values([]))); v && v !== r && n.call(v, a) && (p = v); var g = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(p); function defineIteratorMethods(t) { ["next", "throw", "return"].forEach(function (e) { define(t, e, function (t) { return this._invoke(e, t); }); }); } function AsyncIterator(t, e) { function invoke(r, o, i, a) { var c = tryCatch(t[r], t, o); if ("throw" !== c.type) { var u = c.arg, h = u.value; return h && "object" == _typeof(h) && n.call(h, "__await") ? e.resolve(h.__await).then(function (t) { invoke("next", t, i, a); }, function (t) { invoke("throw", t, i, a); }) : e.resolve(h).then(function (t) { u.value = t, i(u); }, function (t) { return invoke("throw", t, i, a); }); } a(c.arg); } var r; o(this, "_invoke", { value: function value(t, n) { function callInvokeWithMethodAndArg() { return new e(function (e, r) { invoke(t, n, e, r); }); } return r = r ? r.then(callInvokeWithMethodAndArg, callInvokeWithMethodAndArg) : callInvokeWithMethodAndArg(); } }); } function makeInvokeMethod(e, r, n) { var o = h; return function (i, a) { if (o === f) throw Error("Generator is already running"); if (o === s) { if ("throw" === i) throw a; return { value: t, done: !0 }; } for (n.method = i, n.arg = a;;) { var c = n.delegate; if (c) { var u = maybeInvokeDelegate(c, n); if (u) { if (u === y) continue; return u; } } if ("next" === n.method) n.sent = n._sent = n.arg;else if ("throw" === n.method) { if (o === h) throw o = s, n.arg; n.dispatchException(n.arg); } else "return" === n.method && n.abrupt("return", n.arg); o = f; var p = tryCatch(e, r, n); if ("normal" === p.type) { if (o = n.done ? s : l, p.arg === y) continue; return { value: p.arg, done: n.done }; } "throw" === p.type && (o = s, n.method = "throw", n.arg = p.arg); } }; } function maybeInvokeDelegate(e, r) { var n = r.method, o = e.iterator[n]; if (o === t) return r.delegate = null, "throw" === n && e.iterator.return && (r.method = "return", r.arg = t, maybeInvokeDelegate(e, r), "throw" === r.method) || "return" !== n && (r.method = "throw", r.arg = new TypeError("The iterator does not provide a '" + n + "' method")), y; var i = tryCatch(o, e.iterator, r.arg); if ("throw" === i.type) return r.method = "throw", r.arg = i.arg, r.delegate = null, y; var a = i.arg; return a ? a.done ? (r[e.resultName] = a.value, r.next = e.nextLoc, "return" !== r.method && (r.method = "next", r.arg = t), r.delegate = null, y) : a : (r.method = "throw", r.arg = new TypeError("iterator result is not an object"), r.delegate = null, y); } function pushTryEntry(t) { var e = { tryLoc: t[0] }; 1 in t && (e.catchLoc = t[1]), 2 in t && (e.finallyLoc = t[2], e.afterLoc = t[3]), this.tryEntries.push(e); } function resetTryEntry(t) { var e = t.completion || {}; e.type = "normal", delete e.arg, t.completion = e; } function Context(t) { this.tryEntries = [{ tryLoc: "root" }], t.forEach(pushTryEntry, this), this.reset(!0); } function values(e) { if (e || "" === e) { var r = e[a]; if (r) return r.call(e); if ("function" == typeof e.next) return e; if (!isNaN(e.length)) { var o = -1, i = function next() { for (; ++o < e.length;) if (n.call(e, o)) return next.value = e[o], next.done = !1, next; return next.value = t, next.done = !0, next; }; return i.next = i; } } throw new TypeError(_typeof(e) + " is not iterable"); } return GeneratorFunction.prototype = GeneratorFunctionPrototype, o(g, "constructor", { value: GeneratorFunctionPrototype, configurable: !0 }), o(GeneratorFunctionPrototype, "constructor", { value: GeneratorFunction, configurable: !0 }), GeneratorFunction.displayName = define(GeneratorFunctionPrototype, u, "GeneratorFunction"), e.isGeneratorFunction = function (t) { var e = "function" == typeof t && t.constructor; return !!e && (e === GeneratorFunction || "GeneratorFunction" === (e.displayName || e.name)); }, e.mark = function (t) { return Object.setPrototypeOf ? Object.setPrototypeOf(t, GeneratorFunctionPrototype) : (t.__proto__ = GeneratorFunctionPrototype, define(t, u, "GeneratorFunction")), t.prototype = Object.create(g), t; }, e.awrap = function (t) { return { __await: t }; }, defineIteratorMethods(AsyncIterator.prototype), define(AsyncIterator.prototype, c, function () { return this; }), e.AsyncIterator = AsyncIterator, e.async = function (t, r, n, o, i) { void 0 === i && (i = Promise); var a = new AsyncIterator(wrap(t, r, n, o), i); return e.isGeneratorFunction(r) ? a : a.next().then(function (t) { return t.done ? t.value : a.next(); }); }, defineIteratorMethods(g), define(g, u, "Generator"), define(g, a, function () { return this; }), define(g, "toString", function () { return "[object Generator]"; }), e.keys = function (t) { var e = Object(t), r = []; for (var n in e) r.push(n); return r.reverse(), function next() { for (; r.length;) { var t = r.pop(); if (t in e) return next.value = t, next.done = !1, next; } return next.done = !0, next; }; }, e.values = values, Context.prototype = { constructor: Context, reset: function reset(e) { if (this.prev = 0, this.next = 0, this.sent = this._sent = t, this.done = !1, this.delegate = null, this.method = "next", this.arg = t, this.tryEntries.forEach(resetTryEntry), !e) for (var r in this) "t" === r.charAt(0) && n.call(this, r) && !isNaN(+r.slice(1)) && (this[r] = t); }, stop: function stop() { this.done = !0; var t = this.tryEntries[0].completion; if ("throw" === t.type) throw t.arg; return this.rval; }, dispatchException: function dispatchException(e) { if (this.done) throw e; var r = this; function handle(n, o) { return a.type = "throw", a.arg = e, r.next = n, o && (r.method = "next", r.arg = t), !!o; } for (var o = this.tryEntries.length - 1; o >= 0; --o) { var i = this.tryEntries[o], a = i.completion; if ("root" === i.tryLoc) return handle("end"); if (i.tryLoc <= this.prev) { var c = n.call(i, "catchLoc"), u = n.call(i, "finallyLoc"); if (c && u) { if (this.prev < i.catchLoc) return handle(i.catchLoc, !0); if (this.prev < i.finallyLoc) return handle(i.finallyLoc); } else if (c) { if (this.prev < i.catchLoc) return handle(i.catchLoc, !0); } else { if (!u) throw Error("try statement without catch or finally"); if (this.prev < i.finallyLoc) return handle(i.finallyLoc); } } } }, abrupt: function abrupt(t, e) { for (var r = this.tryEntries.length - 1; r >= 0; --r) { var o = this.tryEntries[r]; if (o.tryLoc <= this.prev && n.call(o, "finallyLoc") && this.prev < o.finallyLoc) { var i = o; break; } } i && ("break" === t || "continue" === t) && i.tryLoc <= e && e <= i.finallyLoc && (i = null); var a = i ? i.completion : {}; return a.type = t, a.arg = e, i ? (this.method = "next", this.next = i.finallyLoc, y) : this.complete(a); }, complete: function complete(t, e) { if ("throw" === t.type) throw t.arg; return "break" === t.type || "continue" === t.type ? this.next = t.arg : "return" === t.type ? (this.rval = this.arg = t.arg, this.method = "return", this.next = "end") : "normal" === t.type && e && (this.next = e), y; }, finish: function finish(t) { for (var e = this.tryEntries.length - 1; e >= 0; --e) { var r = this.tryEntries[e]; if (r.finallyLoc === t) return this.complete(r.completion, r.afterLoc), resetTryEntry(r), y; } }, catch: function _catch(t) { for (var e = this.tryEntries.length - 1; e >= 0; --e) { var r = this.tryEntries[e]; if (r.tryLoc === t) { var n = r.completion; if ("throw" === n.type) { var o = n.arg; resetTryEntry(r); } return o; } } throw Error("illegal catch attempt"); }, delegateYield: function delegateYield(e, r, n) { return this.delegate = { iterator: values(e), resultName: r, nextLoc: n }, "next" === this.method && (this.arg = t), y; } }, e; }
function asyncGeneratorStep(n, t, e, r, o, a, c) { try { var i = n[a](c), u = i.value; } catch (n) { return void e(n); } i.done ? t(u) : Promise.resolve(u).then(r, o); }
function _asyncToGenerator(n) { return function () { var t = this, e = arguments; return new Promise(function (r, o) { var a = n.apply(t, e); function _next(n) { asyncGeneratorStep(a, r, o, _next, _throw, "next", n); } function _throw(n) { asyncGeneratorStep(a, r, o, _next, _throw, "throw", n); } _next(void 0); }); }; }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var SUBMIT_TRANSACTION_TIMEOUT = exports.SUBMIT_TRANSACTION_TIMEOUT = 60 * 1000;
var STROOPS_IN_LUMEN = 10000000;
var ACCOUNT_REQUIRES_MEMO = "MQ==";
function getAmountInLumens(amt) {
  return new _bignumber.default(amt).div(STROOPS_IN_LUMEN).toString();
}
var HorizonServer = exports.HorizonServer = function () {
  function HorizonServer(serverURL) {
    var opts = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
    _classCallCheck(this, HorizonServer);
    this.serverURL = (0, _urijs.default)(serverURL);
    var allowHttp = typeof opts.allowHttp === "undefined" ? _config.Config.isAllowHttp() : opts.allowHttp;
    var customHeaders = {};
    if (opts.appName) {
      customHeaders["X-App-Name"] = opts.appName;
    }
    if (opts.appVersion) {
      customHeaders["X-App-Version"] = opts.appVersion;
    }
    if (opts.authToken) {
      customHeaders["X-Auth-Token"] = opts.authToken;
    }
    if (opts.headers) {
      Object.assign(customHeaders, opts.headers);
    }
    if (Object.keys(customHeaders).length > 0) {
      _horizon_axios_client.default.interceptors.request.use(function (config) {
        config.headers = config.headers || {};
        config.headers = Object.assign(config.headers, customHeaders);
        return config;
      });
    }
    if (this.serverURL.protocol() !== "https" && !allowHttp) {
      throw new Error("Cannot connect to insecure horizon server");
    }
  }
  return _createClass(HorizonServer, [{
    key: "fetchTimebounds",
    value: (function () {
      var _fetchTimebounds = _asyncToGenerator(_regeneratorRuntime().mark(function _callee(seconds) {
        var _isRetry,
          currentTime,
          _args = arguments;
        return _regeneratorRuntime().wrap(function _callee$(_context) {
          while (1) switch (_context.prev = _context.next) {
            case 0:
              _isRetry = _args.length > 1 && _args[1] !== undefined ? _args[1] : false;
              currentTime = (0, _horizon_axios_client.getCurrentServerTime)(this.serverURL.hostname());
              if (!currentTime) {
                _context.next = 4;
                break;
              }
              return _context.abrupt("return", {
                minTime: 0,
                maxTime: currentTime + seconds
              });
            case 4:
              if (!_isRetry) {
                _context.next = 6;
                break;
              }
              return _context.abrupt("return", {
                minTime: 0,
                maxTime: Math.floor(new Date().getTime() / 1000) + seconds
              });
            case 6:
              _context.next = 8;
              return _horizon_axios_client.default.get((0, _urijs.default)(this.serverURL).toString());
            case 8:
              return _context.abrupt("return", this.fetchTimebounds(seconds, true));
            case 9:
            case "end":
              return _context.stop();
          }
        }, _callee, this);
      }));
      function fetchTimebounds(_x) {
        return _fetchTimebounds.apply(this, arguments);
      }
      return fetchTimebounds;
    }())
  }, {
    key: "fetchBaseFee",
    value: (function () {
      var _fetchBaseFee = _asyncToGenerator(_regeneratorRuntime().mark(function _callee2() {
        var response;
        return _regeneratorRuntime().wrap(function _callee2$(_context2) {
          while (1) switch (_context2.prev = _context2.next) {
            case 0:
              _context2.next = 2;
              return this.feeStats();
            case 2:
              response = _context2.sent;
              return _context2.abrupt("return", parseInt(response.last_ledger_base_fee, 10) || 100);
            case 4:
            case "end":
              return _context2.stop();
          }
        }, _callee2, this);
      }));
      function fetchBaseFee() {
        return _fetchBaseFee.apply(this, arguments);
      }
      return fetchBaseFee;
    }())
  }, {
    key: "feeStats",
    value: (function () {
      var _feeStats = _asyncToGenerator(_regeneratorRuntime().mark(function _callee3() {
        var cb;
        return _regeneratorRuntime().wrap(function _callee3$(_context3) {
          while (1) switch (_context3.prev = _context3.next) {
            case 0:
              cb = new _call_builder.CallBuilder((0, _urijs.default)(this.serverURL));
              cb.filter.push(["fee_stats"]);
              return _context3.abrupt("return", cb.call());
            case 3:
            case "end":
              return _context3.stop();
          }
        }, _callee3, this);
      }));
      function feeStats() {
        return _feeStats.apply(this, arguments);
      }
      return feeStats;
    }())
  }, {
    key: "root",
    value: (function () {
      var _root = _asyncToGenerator(_regeneratorRuntime().mark(function _callee4() {
        var cb;
        return _regeneratorRuntime().wrap(function _callee4$(_context4) {
          while (1) switch (_context4.prev = _context4.next) {
            case 0:
              cb = new _call_builder.CallBuilder((0, _urijs.default)(this.serverURL));
              return _context4.abrupt("return", cb.call());
            case 2:
            case "end":
              return _context4.stop();
          }
        }, _callee4, this);
      }));
      function root() {
        return _root.apply(this, arguments);
      }
      return root;
    }())
  }, {
    key: "submitTransaction",
    value: (function () {
      var _submitTransaction = _asyncToGenerator(_regeneratorRuntime().mark(function _callee5(transaction) {
        var opts,
          tx,
          _args5 = arguments;
        return _regeneratorRuntime().wrap(function _callee5$(_context5) {
          while (1) switch (_context5.prev = _context5.next) {
            case 0:
              opts = _args5.length > 1 && _args5[1] !== undefined ? _args5[1] : {
                skipMemoRequiredCheck: false
              };
              if (opts.skipMemoRequiredCheck) {
                _context5.next = 4;
                break;
              }
              _context5.next = 4;
              return this.checkMemoRequired(transaction);
            case 4:
              tx = encodeURIComponent(transaction.toEnvelope().toXDR().toString("base64"));
              return _context5.abrupt("return", _horizon_axios_client.default.post((0, _urijs.default)(this.serverURL).segment("transactions").toString(), "tx=".concat(tx), {
                timeout: SUBMIT_TRANSACTION_TIMEOUT
              }).then(function (response) {
                if (!response.data.result_xdr) {
                  return response.data;
                }
                var responseXDR = _stellarBase.xdr.TransactionResult.fromXDR(response.data.result_xdr, "base64");
                var results = responseXDR.result().value();
                var offerResults;
                var hasManageOffer;
                if (results.length) {
                  offerResults = results.map(function (result, i) {
                    if (result.value().switch().name !== "manageBuyOffer" && result.value().switch().name !== "manageSellOffer") {
                      return null;
                    }
                    hasManageOffer = true;
                    var amountBought = new _bignumber.default(0);
                    var amountSold = new _bignumber.default(0);
                    var offerSuccess = result.value().value().success();
                    var offersClaimed = offerSuccess.offersClaimed().map(function (offerClaimedAtom) {
                      var offerClaimed = offerClaimedAtom.value();
                      var sellerId = "";
                      switch (offerClaimedAtom.switch()) {
                        case _stellarBase.xdr.ClaimAtomType.claimAtomTypeV0():
                          sellerId = _stellarBase.StrKey.encodeEd25519PublicKey(offerClaimed.sellerEd25519());
                          break;
                        case _stellarBase.xdr.ClaimAtomType.claimAtomTypeOrderBook():
                          sellerId = _stellarBase.StrKey.encodeEd25519PublicKey(offerClaimed.sellerId().ed25519());
                          break;
                        default:
                          throw new Error("Invalid offer result type: ".concat(offerClaimedAtom.switch()));
                      }
                      var claimedOfferAmountBought = new _bignumber.default(offerClaimed.amountBought().toString());
                      var claimedOfferAmountSold = new _bignumber.default(offerClaimed.amountSold().toString());
                      amountBought = amountBought.plus(claimedOfferAmountSold);
                      amountSold = amountSold.plus(claimedOfferAmountBought);
                      var sold = _stellarBase.Asset.fromOperation(offerClaimed.assetSold());
                      var bought = _stellarBase.Asset.fromOperation(offerClaimed.assetBought());
                      var assetSold = {
                        type: sold.getAssetType(),
                        assetCode: sold.getCode(),
                        issuer: sold.getIssuer()
                      };
                      var assetBought = {
                        type: bought.getAssetType(),
                        assetCode: bought.getCode(),
                        issuer: bought.getIssuer()
                      };
                      return {
                        sellerId: sellerId,
                        offerId: offerClaimed.offerId().toString(),
                        assetSold: assetSold,
                        amountSold: getAmountInLumens(claimedOfferAmountSold),
                        assetBought: assetBought,
                        amountBought: getAmountInLumens(claimedOfferAmountBought)
                      };
                    });
                    var effect = offerSuccess.offer().switch().name;
                    var currentOffer;
                    if (typeof offerSuccess.offer().value === "function" && offerSuccess.offer().value()) {
                      var offerXDR = offerSuccess.offer().value();
                      currentOffer = {
                        offerId: offerXDR.offerId().toString(),
                        selling: {},
                        buying: {},
                        amount: getAmountInLumens(offerXDR.amount().toString()),
                        price: {
                          n: offerXDR.price().n(),
                          d: offerXDR.price().d()
                        }
                      };
                      var selling = _stellarBase.Asset.fromOperation(offerXDR.selling());
                      currentOffer.selling = {
                        type: selling.getAssetType(),
                        assetCode: selling.getCode(),
                        issuer: selling.getIssuer()
                      };
                      var buying = _stellarBase.Asset.fromOperation(offerXDR.buying());
                      currentOffer.buying = {
                        type: buying.getAssetType(),
                        assetCode: buying.getCode(),
                        issuer: buying.getIssuer()
                      };
                    }
                    return {
                      offersClaimed: offersClaimed,
                      effect: effect,
                      operationIndex: i,
                      currentOffer: currentOffer,
                      amountBought: getAmountInLumens(amountBought),
                      amountSold: getAmountInLumens(amountSold),
                      isFullyOpen: !offersClaimed.length && effect !== "manageOfferDeleted",
                      wasPartiallyFilled: !!offersClaimed.length && effect !== "manageOfferDeleted",
                      wasImmediatelyFilled: !!offersClaimed.length && effect === "manageOfferDeleted",
                      wasImmediatelyDeleted: !offersClaimed.length && effect === "manageOfferDeleted"
                    };
                  }).filter(function (result) {
                    return !!result;
                  });
                }
                return _objectSpread(_objectSpread({}, response.data), {}, {
                  offerResults: hasManageOffer ? offerResults : undefined
                });
              }).catch(function (response) {
                if (response instanceof Error) {
                  return Promise.reject(response);
                }
                return Promise.reject(new _errors.BadResponseError("Transaction submission failed. Server responded: ".concat(response.status, " ").concat(response.statusText), response.data));
              }));
            case 6:
            case "end":
              return _context5.stop();
          }
        }, _callee5, this);
      }));
      function submitTransaction(_x2) {
        return _submitTransaction.apply(this, arguments);
      }
      return submitTransaction;
    }())
  }, {
    key: "submitAsyncTransaction",
    value: (function () {
      var _submitAsyncTransaction = _asyncToGenerator(_regeneratorRuntime().mark(function _callee6(transaction) {
        var opts,
          tx,
          _args6 = arguments;
        return _regeneratorRuntime().wrap(function _callee6$(_context6) {
          while (1) switch (_context6.prev = _context6.next) {
            case 0:
              opts = _args6.length > 1 && _args6[1] !== undefined ? _args6[1] : {
                skipMemoRequiredCheck: false
              };
              if (opts.skipMemoRequiredCheck) {
                _context6.next = 4;
                break;
              }
              _context6.next = 4;
              return this.checkMemoRequired(transaction);
            case 4:
              tx = encodeURIComponent(transaction.toEnvelope().toXDR().toString("base64"));
              return _context6.abrupt("return", _horizon_axios_client.default.post((0, _urijs.default)(this.serverURL).segment("transactions_async").toString(), "tx=".concat(tx)).then(function (response) {
                return response.data;
              }).catch(function (response) {
                if (response instanceof Error) {
                  return Promise.reject(response);
                }
                return Promise.reject(new _errors.BadResponseError("Transaction submission failed. Server responded: ".concat(response.status, " ").concat(response.statusText), response.data));
              }));
            case 6:
            case "end":
              return _context6.stop();
          }
        }, _callee6, this);
      }));
      function submitAsyncTransaction(_x3) {
        return _submitAsyncTransaction.apply(this, arguments);
      }
      return submitAsyncTransaction;
    }())
  }, {
    key: "accounts",
    value: function accounts() {
      return new _account_call_builder.AccountCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "claimableBalances",
    value: function claimableBalances() {
      return new _claimable_balances_call_builder.ClaimableBalanceCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "ledgers",
    value: function ledgers() {
      return new _ledger_call_builder.LedgerCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "transactions",
    value: function transactions() {
      return new _transaction_call_builder.TransactionCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "offers",
    value: function offers() {
      return new _offer_call_builder.OfferCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "orderbook",
    value: function orderbook(selling, buying) {
      return new _orderbook_call_builder.OrderbookCallBuilder((0, _urijs.default)(this.serverURL), selling, buying);
    }
  }, {
    key: "trades",
    value: function trades() {
      return new _trades_call_builder.TradesCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "operations",
    value: function operations() {
      return new _operation_call_builder.OperationCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "liquidityPools",
    value: function liquidityPools() {
      return new _liquidity_pool_call_builder.LiquidityPoolCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "strictReceivePaths",
    value: function strictReceivePaths(source, destinationAsset, destinationAmount) {
      return new _strict_receive_path_call_builder.StrictReceivePathCallBuilder((0, _urijs.default)(this.serverURL), source, destinationAsset, destinationAmount);
    }
  }, {
    key: "strictSendPaths",
    value: function strictSendPaths(sourceAsset, sourceAmount, destination) {
      return new _strict_send_path_call_builder.StrictSendPathCallBuilder((0, _urijs.default)(this.serverURL), sourceAsset, sourceAmount, destination);
    }
  }, {
    key: "payments",
    value: function payments() {
      return new _payment_call_builder.PaymentCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "effects",
    value: function effects() {
      return new _effect_call_builder.EffectCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "friendbot",
    value: function friendbot(address) {
      return new _friendbot_builder.FriendbotBuilder((0, _urijs.default)(this.serverURL), address);
    }
  }, {
    key: "assets",
    value: function assets() {
      return new _assets_call_builder.AssetsCallBuilder((0, _urijs.default)(this.serverURL));
    }
  }, {
    key: "loadAccount",
    value: (function () {
      var _loadAccount = _asyncToGenerator(_regeneratorRuntime().mark(function _callee7(accountId) {
        var res;
        return _regeneratorRuntime().wrap(function _callee7$(_context7) {
          while (1) switch (_context7.prev = _context7.next) {
            case 0:
              _context7.next = 2;
              return this.accounts().accountId(accountId).call();
            case 2:
              res = _context7.sent;
              return _context7.abrupt("return", new _account_response.AccountResponse(res));
            case 4:
            case "end":
              return _context7.stop();
          }
        }, _callee7, this);
      }));
      function loadAccount(_x4) {
        return _loadAccount.apply(this, arguments);
      }
      return loadAccount;
    }())
  }, {
    key: "tradeAggregation",
    value: function tradeAggregation(base, counter, start_time, end_time, resolution, offset) {
      return new _trade_aggregation_call_builder.TradeAggregationCallBuilder((0, _urijs.default)(this.serverURL), base, counter, start_time, end_time, resolution, offset);
    }
  }, {
    key: "checkMemoRequired",
    value: (function () {
      var _checkMemoRequired = _asyncToGenerator(_regeneratorRuntime().mark(function _callee8(transaction) {
        var destinations, i, operation, destination, account;
        return _regeneratorRuntime().wrap(function _callee8$(_context8) {
          while (1) switch (_context8.prev = _context8.next) {
            case 0:
              if (transaction instanceof _stellarBase.FeeBumpTransaction) {
                transaction = transaction.innerTransaction;
              }
              if (!(transaction.memo.type !== "none")) {
                _context8.next = 3;
                break;
              }
              return _context8.abrupt("return");
            case 3:
              destinations = new Set();
              i = 0;
            case 5:
              if (!(i < transaction.operations.length)) {
                _context8.next = 36;
                break;
              }
              operation = transaction.operations[i];
              _context8.t0 = operation.type;
              _context8.next = _context8.t0 === "payment" ? 10 : _context8.t0 === "pathPaymentStrictReceive" ? 10 : _context8.t0 === "pathPaymentStrictSend" ? 10 : _context8.t0 === "accountMerge" ? 10 : 11;
              break;
            case 10:
              return _context8.abrupt("break", 12);
            case 11:
              return _context8.abrupt("continue", 33);
            case 12:
              destination = operation.destination;
              if (!destinations.has(destination)) {
                _context8.next = 15;
                break;
              }
              return _context8.abrupt("continue", 33);
            case 15:
              destinations.add(destination);
              if (!destination.startsWith("M")) {
                _context8.next = 18;
                break;
              }
              return _context8.abrupt("continue", 33);
            case 18:
              _context8.prev = 18;
              _context8.next = 21;
              return this.loadAccount(destination);
            case 21:
              account = _context8.sent;
              if (!(account.data_attr["config.memo_required"] === ACCOUNT_REQUIRES_MEMO)) {
                _context8.next = 24;
                break;
              }
              throw new _errors.AccountRequiresMemoError("account requires memo", destination, i);
            case 24:
              _context8.next = 33;
              break;
            case 26:
              _context8.prev = 26;
              _context8.t1 = _context8["catch"](18);
              if (!(_context8.t1 instanceof _errors.AccountRequiresMemoError)) {
                _context8.next = 30;
                break;
              }
              throw _context8.t1;
            case 30:
              if (_context8.t1 instanceof _errors.NotFoundError) {
                _context8.next = 32;
                break;
              }
              throw _context8.t1;
            case 32:
              return _context8.abrupt("continue", 33);
            case 33:
              i += 1;
              _context8.next = 5;
              break;
            case 36:
            case "end":
              return _context8.stop();
          }
        }, _callee8, this, [[18, 26]]);
      }));
      function checkMemoRequired(_x5) {
        return _checkMemoRequired.apply(this, arguments);
      }
      return checkMemoRequired;
    }())
  }]);
}();