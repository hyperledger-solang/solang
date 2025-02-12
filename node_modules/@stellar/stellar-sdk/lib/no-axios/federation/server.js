"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.FederationServer = exports.FEDERATION_RESPONSE_MAX_SIZE = void 0;
var _stellarBase = require("@stellar/stellar-base");
var _urijs = _interopRequireDefault(require("urijs"));
var _config = require("../config");
var _errors = require("../errors");
var _stellartoml = require("../stellartoml");
var _httpClient = require("../http-client");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
function _regeneratorRuntime() { "use strict"; _regeneratorRuntime = function _regeneratorRuntime() { return e; }; var t, e = {}, r = Object.prototype, n = r.hasOwnProperty, o = Object.defineProperty || function (t, e, r) { t[e] = r.value; }, i = "function" == typeof Symbol ? Symbol : {}, a = i.iterator || "@@iterator", c = i.asyncIterator || "@@asyncIterator", u = i.toStringTag || "@@toStringTag"; function define(t, e, r) { return Object.defineProperty(t, e, { value: r, enumerable: !0, configurable: !0, writable: !0 }), t[e]; } try { define({}, ""); } catch (t) { define = function define(t, e, r) { return t[e] = r; }; } function wrap(t, e, r, n) { var i = e && e.prototype instanceof Generator ? e : Generator, a = Object.create(i.prototype), c = new Context(n || []); return o(a, "_invoke", { value: makeInvokeMethod(t, r, c) }), a; } function tryCatch(t, e, r) { try { return { type: "normal", arg: t.call(e, r) }; } catch (t) { return { type: "throw", arg: t }; } } e.wrap = wrap; var h = "suspendedStart", l = "suspendedYield", f = "executing", s = "completed", y = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} var p = {}; define(p, a, function () { return this; }); var d = Object.getPrototypeOf, v = d && d(d(values([]))); v && v !== r && n.call(v, a) && (p = v); var g = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(p); function defineIteratorMethods(t) { ["next", "throw", "return"].forEach(function (e) { define(t, e, function (t) { return this._invoke(e, t); }); }); } function AsyncIterator(t, e) { function invoke(r, o, i, a) { var c = tryCatch(t[r], t, o); if ("throw" !== c.type) { var u = c.arg, h = u.value; return h && "object" == _typeof(h) && n.call(h, "__await") ? e.resolve(h.__await).then(function (t) { invoke("next", t, i, a); }, function (t) { invoke("throw", t, i, a); }) : e.resolve(h).then(function (t) { u.value = t, i(u); }, function (t) { return invoke("throw", t, i, a); }); } a(c.arg); } var r; o(this, "_invoke", { value: function value(t, n) { function callInvokeWithMethodAndArg() { return new e(function (e, r) { invoke(t, n, e, r); }); } return r = r ? r.then(callInvokeWithMethodAndArg, callInvokeWithMethodAndArg) : callInvokeWithMethodAndArg(); } }); } function makeInvokeMethod(e, r, n) { var o = h; return function (i, a) { if (o === f) throw Error("Generator is already running"); if (o === s) { if ("throw" === i) throw a; return { value: t, done: !0 }; } for (n.method = i, n.arg = a;;) { var c = n.delegate; if (c) { var u = maybeInvokeDelegate(c, n); if (u) { if (u === y) continue; return u; } } if ("next" === n.method) n.sent = n._sent = n.arg;else if ("throw" === n.method) { if (o === h) throw o = s, n.arg; n.dispatchException(n.arg); } else "return" === n.method && n.abrupt("return", n.arg); o = f; var p = tryCatch(e, r, n); if ("normal" === p.type) { if (o = n.done ? s : l, p.arg === y) continue; return { value: p.arg, done: n.done }; } "throw" === p.type && (o = s, n.method = "throw", n.arg = p.arg); } }; } function maybeInvokeDelegate(e, r) { var n = r.method, o = e.iterator[n]; if (o === t) return r.delegate = null, "throw" === n && e.iterator.return && (r.method = "return", r.arg = t, maybeInvokeDelegate(e, r), "throw" === r.method) || "return" !== n && (r.method = "throw", r.arg = new TypeError("The iterator does not provide a '" + n + "' method")), y; var i = tryCatch(o, e.iterator, r.arg); if ("throw" === i.type) return r.method = "throw", r.arg = i.arg, r.delegate = null, y; var a = i.arg; return a ? a.done ? (r[e.resultName] = a.value, r.next = e.nextLoc, "return" !== r.method && (r.method = "next", r.arg = t), r.delegate = null, y) : a : (r.method = "throw", r.arg = new TypeError("iterator result is not an object"), r.delegate = null, y); } function pushTryEntry(t) { var e = { tryLoc: t[0] }; 1 in t && (e.catchLoc = t[1]), 2 in t && (e.finallyLoc = t[2], e.afterLoc = t[3]), this.tryEntries.push(e); } function resetTryEntry(t) { var e = t.completion || {}; e.type = "normal", delete e.arg, t.completion = e; } function Context(t) { this.tryEntries = [{ tryLoc: "root" }], t.forEach(pushTryEntry, this), this.reset(!0); } function values(e) { if (e || "" === e) { var r = e[a]; if (r) return r.call(e); if ("function" == typeof e.next) return e; if (!isNaN(e.length)) { var o = -1, i = function next() { for (; ++o < e.length;) if (n.call(e, o)) return next.value = e[o], next.done = !1, next; return next.value = t, next.done = !0, next; }; return i.next = i; } } throw new TypeError(_typeof(e) + " is not iterable"); } return GeneratorFunction.prototype = GeneratorFunctionPrototype, o(g, "constructor", { value: GeneratorFunctionPrototype, configurable: !0 }), o(GeneratorFunctionPrototype, "constructor", { value: GeneratorFunction, configurable: !0 }), GeneratorFunction.displayName = define(GeneratorFunctionPrototype, u, "GeneratorFunction"), e.isGeneratorFunction = function (t) { var e = "function" == typeof t && t.constructor; return !!e && (e === GeneratorFunction || "GeneratorFunction" === (e.displayName || e.name)); }, e.mark = function (t) { return Object.setPrototypeOf ? Object.setPrototypeOf(t, GeneratorFunctionPrototype) : (t.__proto__ = GeneratorFunctionPrototype, define(t, u, "GeneratorFunction")), t.prototype = Object.create(g), t; }, e.awrap = function (t) { return { __await: t }; }, defineIteratorMethods(AsyncIterator.prototype), define(AsyncIterator.prototype, c, function () { return this; }), e.AsyncIterator = AsyncIterator, e.async = function (t, r, n, o, i) { void 0 === i && (i = Promise); var a = new AsyncIterator(wrap(t, r, n, o), i); return e.isGeneratorFunction(r) ? a : a.next().then(function (t) { return t.done ? t.value : a.next(); }); }, defineIteratorMethods(g), define(g, u, "Generator"), define(g, a, function () { return this; }), define(g, "toString", function () { return "[object Generator]"; }), e.keys = function (t) { var e = Object(t), r = []; for (var n in e) r.push(n); return r.reverse(), function next() { for (; r.length;) { var t = r.pop(); if (t in e) return next.value = t, next.done = !1, next; } return next.done = !0, next; }; }, e.values = values, Context.prototype = { constructor: Context, reset: function reset(e) { if (this.prev = 0, this.next = 0, this.sent = this._sent = t, this.done = !1, this.delegate = null, this.method = "next", this.arg = t, this.tryEntries.forEach(resetTryEntry), !e) for (var r in this) "t" === r.charAt(0) && n.call(this, r) && !isNaN(+r.slice(1)) && (this[r] = t); }, stop: function stop() { this.done = !0; var t = this.tryEntries[0].completion; if ("throw" === t.type) throw t.arg; return this.rval; }, dispatchException: function dispatchException(e) { if (this.done) throw e; var r = this; function handle(n, o) { return a.type = "throw", a.arg = e, r.next = n, o && (r.method = "next", r.arg = t), !!o; } for (var o = this.tryEntries.length - 1; o >= 0; --o) { var i = this.tryEntries[o], a = i.completion; if ("root" === i.tryLoc) return handle("end"); if (i.tryLoc <= this.prev) { var c = n.call(i, "catchLoc"), u = n.call(i, "finallyLoc"); if (c && u) { if (this.prev < i.catchLoc) return handle(i.catchLoc, !0); if (this.prev < i.finallyLoc) return handle(i.finallyLoc); } else if (c) { if (this.prev < i.catchLoc) return handle(i.catchLoc, !0); } else { if (!u) throw Error("try statement without catch or finally"); if (this.prev < i.finallyLoc) return handle(i.finallyLoc); } } } }, abrupt: function abrupt(t, e) { for (var r = this.tryEntries.length - 1; r >= 0; --r) { var o = this.tryEntries[r]; if (o.tryLoc <= this.prev && n.call(o, "finallyLoc") && this.prev < o.finallyLoc) { var i = o; break; } } i && ("break" === t || "continue" === t) && i.tryLoc <= e && e <= i.finallyLoc && (i = null); var a = i ? i.completion : {}; return a.type = t, a.arg = e, i ? (this.method = "next", this.next = i.finallyLoc, y) : this.complete(a); }, complete: function complete(t, e) { if ("throw" === t.type) throw t.arg; return "break" === t.type || "continue" === t.type ? this.next = t.arg : "return" === t.type ? (this.rval = this.arg = t.arg, this.method = "return", this.next = "end") : "normal" === t.type && e && (this.next = e), y; }, finish: function finish(t) { for (var e = this.tryEntries.length - 1; e >= 0; --e) { var r = this.tryEntries[e]; if (r.finallyLoc === t) return this.complete(r.completion, r.afterLoc), resetTryEntry(r), y; } }, catch: function _catch(t) { for (var e = this.tryEntries.length - 1; e >= 0; --e) { var r = this.tryEntries[e]; if (r.tryLoc === t) { var n = r.completion; if ("throw" === n.type) { var o = n.arg; resetTryEntry(r); } return o; } } throw Error("illegal catch attempt"); }, delegateYield: function delegateYield(e, r, n) { return this.delegate = { iterator: values(e), resultName: r, nextLoc: n }, "next" === this.method && (this.arg = t), y; } }, e; }
function asyncGeneratorStep(n, t, e, r, o, a, c) { try { var i = n[a](c), u = i.value; } catch (n) { return void e(n); } i.done ? t(u) : Promise.resolve(u).then(r, o); }
function _asyncToGenerator(n) { return function () { var t = this, e = arguments; return new Promise(function (r, o) { var a = n.apply(t, e); function _next(n) { asyncGeneratorStep(a, r, o, _next, _throw, "next", n); } function _throw(n) { asyncGeneratorStep(a, r, o, _next, _throw, "throw", n); } _next(void 0); }); }; }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var FEDERATION_RESPONSE_MAX_SIZE = exports.FEDERATION_RESPONSE_MAX_SIZE = 100 * 1024;
var FederationServer = exports.FederationServer = function () {
  function FederationServer(serverURL, domain) {
    var opts = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : {};
    _classCallCheck(this, FederationServer);
    this.serverURL = (0, _urijs.default)(serverURL);
    this.domain = domain;
    var allowHttp = typeof opts.allowHttp === "undefined" ? _config.Config.isAllowHttp() : opts.allowHttp;
    this.timeout = typeof opts.timeout === "undefined" ? _config.Config.getTimeout() : opts.timeout;
    if (this.serverURL.protocol() !== "https" && !allowHttp) {
      throw new Error("Cannot connect to insecure federation server");
    }
  }
  return _createClass(FederationServer, [{
    key: "resolveAddress",
    value: (function () {
      var _resolveAddress = _asyncToGenerator(_regeneratorRuntime().mark(function _callee(address) {
        var stellarAddress, url;
        return _regeneratorRuntime().wrap(function _callee$(_context) {
          while (1) switch (_context.prev = _context.next) {
            case 0:
              stellarAddress = address;
              if (!(address.indexOf("*") < 0)) {
                _context.next = 5;
                break;
              }
              if (this.domain) {
                _context.next = 4;
                break;
              }
              return _context.abrupt("return", Promise.reject(new Error("Unknown domain. Make sure `address` contains a domain (ex. `bob*stellar.org`) or pass `domain` parameter when instantiating the server object.")));
            case 4:
              stellarAddress = "".concat(address, "*").concat(this.domain);
            case 5:
              url = this.serverURL.query({
                type: "name",
                q: stellarAddress
              });
              return _context.abrupt("return", this._sendRequest(url));
            case 7:
            case "end":
              return _context.stop();
          }
        }, _callee, this);
      }));
      function resolveAddress(_x) {
        return _resolveAddress.apply(this, arguments);
      }
      return resolveAddress;
    }())
  }, {
    key: "resolveAccountId",
    value: (function () {
      var _resolveAccountId = _asyncToGenerator(_regeneratorRuntime().mark(function _callee2(accountId) {
        var url;
        return _regeneratorRuntime().wrap(function _callee2$(_context2) {
          while (1) switch (_context2.prev = _context2.next) {
            case 0:
              url = this.serverURL.query({
                type: "id",
                q: accountId
              });
              return _context2.abrupt("return", this._sendRequest(url));
            case 2:
            case "end":
              return _context2.stop();
          }
        }, _callee2, this);
      }));
      function resolveAccountId(_x2) {
        return _resolveAccountId.apply(this, arguments);
      }
      return resolveAccountId;
    }())
  }, {
    key: "resolveTransactionId",
    value: (function () {
      var _resolveTransactionId = _asyncToGenerator(_regeneratorRuntime().mark(function _callee3(transactionId) {
        var url;
        return _regeneratorRuntime().wrap(function _callee3$(_context3) {
          while (1) switch (_context3.prev = _context3.next) {
            case 0:
              url = this.serverURL.query({
                type: "txid",
                q: transactionId
              });
              return _context3.abrupt("return", this._sendRequest(url));
            case 2:
            case "end":
              return _context3.stop();
          }
        }, _callee3, this);
      }));
      function resolveTransactionId(_x3) {
        return _resolveTransactionId.apply(this, arguments);
      }
      return resolveTransactionId;
    }())
  }, {
    key: "_sendRequest",
    value: function () {
      var _sendRequest2 = _asyncToGenerator(_regeneratorRuntime().mark(function _callee4(url) {
        var timeout;
        return _regeneratorRuntime().wrap(function _callee4$(_context4) {
          while (1) switch (_context4.prev = _context4.next) {
            case 0:
              timeout = this.timeout;
              return _context4.abrupt("return", _httpClient.httpClient.get(url.toString(), {
                maxContentLength: FEDERATION_RESPONSE_MAX_SIZE,
                timeout: timeout
              }).then(function (response) {
                if (typeof response.data.memo !== "undefined" && typeof response.data.memo !== "string") {
                  throw new Error("memo value should be of type string");
                }
                return response.data;
              }).catch(function (response) {
                if (response instanceof Error) {
                  if (response.message.match(/^maxContentLength size/)) {
                    throw new Error("federation response exceeds allowed size of ".concat(FEDERATION_RESPONSE_MAX_SIZE));
                  } else {
                    return Promise.reject(response);
                  }
                } else {
                  return Promise.reject(new _errors.BadResponseError("Server query failed. Server responded: ".concat(response.status, " ").concat(response.statusText), response.data));
                }
              }));
            case 2:
            case "end":
              return _context4.stop();
          }
        }, _callee4, this);
      }));
      function _sendRequest(_x4) {
        return _sendRequest2.apply(this, arguments);
      }
      return _sendRequest;
    }()
  }], [{
    key: "resolve",
    value: (function () {
      var _resolve = _asyncToGenerator(_regeneratorRuntime().mark(function _callee5(value) {
        var opts,
          addressParts,
          _addressParts,
          domain,
          federationServer,
          _args5 = arguments;
        return _regeneratorRuntime().wrap(function _callee5$(_context5) {
          while (1) switch (_context5.prev = _context5.next) {
            case 0:
              opts = _args5.length > 1 && _args5[1] !== undefined ? _args5[1] : {};
              if (!(value.indexOf("*") < 0)) {
                _context5.next = 5;
                break;
              }
              if (_stellarBase.StrKey.isValidEd25519PublicKey(value)) {
                _context5.next = 4;
                break;
              }
              return _context5.abrupt("return", Promise.reject(new Error("Invalid Account ID")));
            case 4:
              return _context5.abrupt("return", Promise.resolve({
                account_id: value
              }));
            case 5:
              addressParts = value.split("*");
              _addressParts = _slicedToArray(addressParts, 2), domain = _addressParts[1];
              if (!(addressParts.length !== 2 || !domain)) {
                _context5.next = 9;
                break;
              }
              return _context5.abrupt("return", Promise.reject(new Error("Invalid Stellar address")));
            case 9:
              _context5.next = 11;
              return FederationServer.createForDomain(domain, opts);
            case 11:
              federationServer = _context5.sent;
              return _context5.abrupt("return", federationServer.resolveAddress(value));
            case 13:
            case "end":
              return _context5.stop();
          }
        }, _callee5);
      }));
      function resolve(_x5) {
        return _resolve.apply(this, arguments);
      }
      return resolve;
    }())
  }, {
    key: "createForDomain",
    value: (function () {
      var _createForDomain = _asyncToGenerator(_regeneratorRuntime().mark(function _callee6(domain) {
        var opts,
          tomlObject,
          _args6 = arguments;
        return _regeneratorRuntime().wrap(function _callee6$(_context6) {
          while (1) switch (_context6.prev = _context6.next) {
            case 0:
              opts = _args6.length > 1 && _args6[1] !== undefined ? _args6[1] : {};
              _context6.next = 3;
              return _stellartoml.Resolver.resolve(domain, opts);
            case 3:
              tomlObject = _context6.sent;
              if (tomlObject.FEDERATION_SERVER) {
                _context6.next = 6;
                break;
              }
              return _context6.abrupt("return", Promise.reject(new Error("stellar.toml does not contain FEDERATION_SERVER field")));
            case 6:
              return _context6.abrupt("return", new FederationServer(tomlObject.FEDERATION_SERVER, domain, opts));
            case 7:
            case "end":
              return _context6.stop();
          }
        }, _callee6);
      }));
      function createForDomain(_x6) {
        return _createForDomain.apply(this, arguments);
      }
      return createForDomain;
    }())
  }]);
}();