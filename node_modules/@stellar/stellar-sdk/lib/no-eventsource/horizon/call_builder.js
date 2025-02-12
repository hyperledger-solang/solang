"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.CallBuilder = void 0;
var _urijs = _interopRequireDefault(require("urijs"));
var _URITemplate = _interopRequireDefault(require("urijs/src/URITemplate"));
var _errors = require("../errors");
var _horizon_axios_client = require("./horizon_axios_client");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function _regeneratorRuntime() { "use strict"; _regeneratorRuntime = function _regeneratorRuntime() { return e; }; var t, e = {}, r = Object.prototype, n = r.hasOwnProperty, o = Object.defineProperty || function (t, e, r) { t[e] = r.value; }, i = "function" == typeof Symbol ? Symbol : {}, a = i.iterator || "@@iterator", c = i.asyncIterator || "@@asyncIterator", u = i.toStringTag || "@@toStringTag"; function define(t, e, r) { return Object.defineProperty(t, e, { value: r, enumerable: !0, configurable: !0, writable: !0 }), t[e]; } try { define({}, ""); } catch (t) { define = function define(t, e, r) { return t[e] = r; }; } function wrap(t, e, r, n) { var i = e && e.prototype instanceof Generator ? e : Generator, a = Object.create(i.prototype), c = new Context(n || []); return o(a, "_invoke", { value: makeInvokeMethod(t, r, c) }), a; } function tryCatch(t, e, r) { try { return { type: "normal", arg: t.call(e, r) }; } catch (t) { return { type: "throw", arg: t }; } } e.wrap = wrap; var h = "suspendedStart", l = "suspendedYield", f = "executing", s = "completed", y = {}; function Generator() {} function GeneratorFunction() {} function GeneratorFunctionPrototype() {} var p = {}; define(p, a, function () { return this; }); var d = Object.getPrototypeOf, v = d && d(d(values([]))); v && v !== r && n.call(v, a) && (p = v); var g = GeneratorFunctionPrototype.prototype = Generator.prototype = Object.create(p); function defineIteratorMethods(t) { ["next", "throw", "return"].forEach(function (e) { define(t, e, function (t) { return this._invoke(e, t); }); }); } function AsyncIterator(t, e) { function invoke(r, o, i, a) { var c = tryCatch(t[r], t, o); if ("throw" !== c.type) { var u = c.arg, h = u.value; return h && "object" == _typeof(h) && n.call(h, "__await") ? e.resolve(h.__await).then(function (t) { invoke("next", t, i, a); }, function (t) { invoke("throw", t, i, a); }) : e.resolve(h).then(function (t) { u.value = t, i(u); }, function (t) { return invoke("throw", t, i, a); }); } a(c.arg); } var r; o(this, "_invoke", { value: function value(t, n) { function callInvokeWithMethodAndArg() { return new e(function (e, r) { invoke(t, n, e, r); }); } return r = r ? r.then(callInvokeWithMethodAndArg, callInvokeWithMethodAndArg) : callInvokeWithMethodAndArg(); } }); } function makeInvokeMethod(e, r, n) { var o = h; return function (i, a) { if (o === f) throw Error("Generator is already running"); if (o === s) { if ("throw" === i) throw a; return { value: t, done: !0 }; } for (n.method = i, n.arg = a;;) { var c = n.delegate; if (c) { var u = maybeInvokeDelegate(c, n); if (u) { if (u === y) continue; return u; } } if ("next" === n.method) n.sent = n._sent = n.arg;else if ("throw" === n.method) { if (o === h) throw o = s, n.arg; n.dispatchException(n.arg); } else "return" === n.method && n.abrupt("return", n.arg); o = f; var p = tryCatch(e, r, n); if ("normal" === p.type) { if (o = n.done ? s : l, p.arg === y) continue; return { value: p.arg, done: n.done }; } "throw" === p.type && (o = s, n.method = "throw", n.arg = p.arg); } }; } function maybeInvokeDelegate(e, r) { var n = r.method, o = e.iterator[n]; if (o === t) return r.delegate = null, "throw" === n && e.iterator.return && (r.method = "return", r.arg = t, maybeInvokeDelegate(e, r), "throw" === r.method) || "return" !== n && (r.method = "throw", r.arg = new TypeError("The iterator does not provide a '" + n + "' method")), y; var i = tryCatch(o, e.iterator, r.arg); if ("throw" === i.type) return r.method = "throw", r.arg = i.arg, r.delegate = null, y; var a = i.arg; return a ? a.done ? (r[e.resultName] = a.value, r.next = e.nextLoc, "return" !== r.method && (r.method = "next", r.arg = t), r.delegate = null, y) : a : (r.method = "throw", r.arg = new TypeError("iterator result is not an object"), r.delegate = null, y); } function pushTryEntry(t) { var e = { tryLoc: t[0] }; 1 in t && (e.catchLoc = t[1]), 2 in t && (e.finallyLoc = t[2], e.afterLoc = t[3]), this.tryEntries.push(e); } function resetTryEntry(t) { var e = t.completion || {}; e.type = "normal", delete e.arg, t.completion = e; } function Context(t) { this.tryEntries = [{ tryLoc: "root" }], t.forEach(pushTryEntry, this), this.reset(!0); } function values(e) { if (e || "" === e) { var r = e[a]; if (r) return r.call(e); if ("function" == typeof e.next) return e; if (!isNaN(e.length)) { var o = -1, i = function next() { for (; ++o < e.length;) if (n.call(e, o)) return next.value = e[o], next.done = !1, next; return next.value = t, next.done = !0, next; }; return i.next = i; } } throw new TypeError(_typeof(e) + " is not iterable"); } return GeneratorFunction.prototype = GeneratorFunctionPrototype, o(g, "constructor", { value: GeneratorFunctionPrototype, configurable: !0 }), o(GeneratorFunctionPrototype, "constructor", { value: GeneratorFunction, configurable: !0 }), GeneratorFunction.displayName = define(GeneratorFunctionPrototype, u, "GeneratorFunction"), e.isGeneratorFunction = function (t) { var e = "function" == typeof t && t.constructor; return !!e && (e === GeneratorFunction || "GeneratorFunction" === (e.displayName || e.name)); }, e.mark = function (t) { return Object.setPrototypeOf ? Object.setPrototypeOf(t, GeneratorFunctionPrototype) : (t.__proto__ = GeneratorFunctionPrototype, define(t, u, "GeneratorFunction")), t.prototype = Object.create(g), t; }, e.awrap = function (t) { return { __await: t }; }, defineIteratorMethods(AsyncIterator.prototype), define(AsyncIterator.prototype, c, function () { return this; }), e.AsyncIterator = AsyncIterator, e.async = function (t, r, n, o, i) { void 0 === i && (i = Promise); var a = new AsyncIterator(wrap(t, r, n, o), i); return e.isGeneratorFunction(r) ? a : a.next().then(function (t) { return t.done ? t.value : a.next(); }); }, defineIteratorMethods(g), define(g, u, "Generator"), define(g, a, function () { return this; }), define(g, "toString", function () { return "[object Generator]"; }), e.keys = function (t) { var e = Object(t), r = []; for (var n in e) r.push(n); return r.reverse(), function next() { for (; r.length;) { var t = r.pop(); if (t in e) return next.value = t, next.done = !1, next; } return next.done = !0, next; }; }, e.values = values, Context.prototype = { constructor: Context, reset: function reset(e) { if (this.prev = 0, this.next = 0, this.sent = this._sent = t, this.done = !1, this.delegate = null, this.method = "next", this.arg = t, this.tryEntries.forEach(resetTryEntry), !e) for (var r in this) "t" === r.charAt(0) && n.call(this, r) && !isNaN(+r.slice(1)) && (this[r] = t); }, stop: function stop() { this.done = !0; var t = this.tryEntries[0].completion; if ("throw" === t.type) throw t.arg; return this.rval; }, dispatchException: function dispatchException(e) { if (this.done) throw e; var r = this; function handle(n, o) { return a.type = "throw", a.arg = e, r.next = n, o && (r.method = "next", r.arg = t), !!o; } for (var o = this.tryEntries.length - 1; o >= 0; --o) { var i = this.tryEntries[o], a = i.completion; if ("root" === i.tryLoc) return handle("end"); if (i.tryLoc <= this.prev) { var c = n.call(i, "catchLoc"), u = n.call(i, "finallyLoc"); if (c && u) { if (this.prev < i.catchLoc) return handle(i.catchLoc, !0); if (this.prev < i.finallyLoc) return handle(i.finallyLoc); } else if (c) { if (this.prev < i.catchLoc) return handle(i.catchLoc, !0); } else { if (!u) throw Error("try statement without catch or finally"); if (this.prev < i.finallyLoc) return handle(i.finallyLoc); } } } }, abrupt: function abrupt(t, e) { for (var r = this.tryEntries.length - 1; r >= 0; --r) { var o = this.tryEntries[r]; if (o.tryLoc <= this.prev && n.call(o, "finallyLoc") && this.prev < o.finallyLoc) { var i = o; break; } } i && ("break" === t || "continue" === t) && i.tryLoc <= e && e <= i.finallyLoc && (i = null); var a = i ? i.completion : {}; return a.type = t, a.arg = e, i ? (this.method = "next", this.next = i.finallyLoc, y) : this.complete(a); }, complete: function complete(t, e) { if ("throw" === t.type) throw t.arg; return "break" === t.type || "continue" === t.type ? this.next = t.arg : "return" === t.type ? (this.rval = this.arg = t.arg, this.method = "return", this.next = "end") : "normal" === t.type && e && (this.next = e), y; }, finish: function finish(t) { for (var e = this.tryEntries.length - 1; e >= 0; --e) { var r = this.tryEntries[e]; if (r.finallyLoc === t) return this.complete(r.completion, r.afterLoc), resetTryEntry(r), y; } }, catch: function _catch(t) { for (var e = this.tryEntries.length - 1; e >= 0; --e) { var r = this.tryEntries[e]; if (r.tryLoc === t) { var n = r.completion; if ("throw" === n.type) { var o = n.arg; resetTryEntry(r); } return o; } } throw Error("illegal catch attempt"); }, delegateYield: function delegateYield(e, r, n) { return this.delegate = { iterator: values(e), resultName: r, nextLoc: n }, "next" === this.method && (this.arg = t), y; } }, e; }
function asyncGeneratorStep(n, t, e, r, o, a, c) { try { var i = n[a](c), u = i.value; } catch (n) { return void e(n); } i.done ? t(u) : Promise.resolve(u).then(r, o); }
function _asyncToGenerator(n) { return function () { var t = this, e = arguments; return new Promise(function (r, o) { var a = n.apply(t, e); function _next(n) { asyncGeneratorStep(a, r, o, _next, _throw, "next", n); } function _throw(n) { asyncGeneratorStep(a, r, o, _next, _throw, "throw", n); } _next(void 0); }); }; }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var JOINABLE = ["transaction"];
var anyGlobal = global;
var EventSource;
if (typeof false !== 'undefined' && false) {
  var _ref, _anyGlobal$EventSourc, _anyGlobal$window;
  EventSource = (_ref = (_anyGlobal$EventSourc = anyGlobal.EventSource) !== null && _anyGlobal$EventSourc !== void 0 ? _anyGlobal$EventSourc : (_anyGlobal$window = anyGlobal.window) === null || _anyGlobal$window === void 0 ? void 0 : _anyGlobal$window.EventSource) !== null && _ref !== void 0 ? _ref : require("eventsource");
}
var CallBuilder = exports.CallBuilder = function () {
  function CallBuilder(serverUrl) {
    var neighborRoot = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : "";
    _classCallCheck(this, CallBuilder);
    this.url = serverUrl.clone();
    this.filter = [];
    this.originalSegments = this.url.segment() || [];
    this.neighborRoot = neighborRoot;
  }
  return _createClass(CallBuilder, [{
    key: "call",
    value: function call() {
      var _this = this;
      this.checkFilter();
      return this._sendNormalRequest(this.url).then(function (r) {
        return _this._parseResponse(r);
      });
    }
  }, {
    key: "stream",
    value: function stream() {
      var _this2 = this;
      var options = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
      if (EventSource === undefined) {
        throw new Error("Streaming requires eventsource to be enabled. If you need this functionality, compile with USE_EVENTSOURCE=true.");
      }
      this.checkFilter();
      this.url.setQuery("X-Client-Name", "js-stellar-sdk");
      this.url.setQuery("X-Client-Version", _horizon_axios_client.version);
      var es;
      var timeout;
      var createTimeout = function createTimeout() {
        timeout = setTimeout(function () {
          var _es;
          (_es = es) === null || _es === void 0 || _es.close();
          es = _createEventSource();
        }, options.reconnectTimeout || 15 * 1000);
      };
      var _createEventSource = function createEventSource() {
        try {
          es = new EventSource(_this2.url.toString());
        } catch (err) {
          if (options.onerror) {
            options.onerror(err);
          }
        }
        createTimeout();
        if (!es) {
          return es;
        }
        var closed = false;
        var onClose = function onClose() {
          if (closed) {
            return;
          }
          clearTimeout(timeout);
          es.close();
          _createEventSource();
          closed = true;
        };
        var onMessage = function onMessage(message) {
          if (message.type === "close") {
            onClose();
            return;
          }
          var result = message.data ? _this2._parseRecord(JSON.parse(message.data)) : message;
          if (result.paging_token) {
            _this2.url.setQuery("cursor", result.paging_token);
          }
          clearTimeout(timeout);
          createTimeout();
          if (typeof options.onmessage !== "undefined") {
            options.onmessage(result);
          }
        };
        var onError = function onError(error) {
          if (options.onerror) {
            options.onerror(error);
          }
        };
        if (es.addEventListener) {
          es.addEventListener("message", onMessage.bind(_this2));
          es.addEventListener("error", onError.bind(_this2));
          es.addEventListener("close", onClose.bind(_this2));
        } else {
          es.onmessage = onMessage.bind(_this2);
          es.onerror = onError.bind(_this2);
        }
        return es;
      };
      _createEventSource();
      return function () {
        var _es2;
        clearTimeout(timeout);
        (_es2 = es) === null || _es2 === void 0 || _es2.close();
      };
    }
  }, {
    key: "cursor",
    value: function cursor(_cursor) {
      this.url.setQuery("cursor", _cursor);
      return this;
    }
  }, {
    key: "limit",
    value: function limit(recordsNumber) {
      this.url.setQuery("limit", recordsNumber.toString());
      return this;
    }
  }, {
    key: "order",
    value: function order(direction) {
      this.url.setQuery("order", direction);
      return this;
    }
  }, {
    key: "join",
    value: function join(include) {
      this.url.setQuery("join", include);
      return this;
    }
  }, {
    key: "forEndpoint",
    value: function forEndpoint(endpoint, param) {
      if (this.neighborRoot === "") {
        throw new Error("Invalid usage: neighborRoot not set in constructor");
      }
      this.filter.push([endpoint, param, this.neighborRoot]);
      return this;
    }
  }, {
    key: "checkFilter",
    value: function checkFilter() {
      if (this.filter.length >= 2) {
        throw new _errors.BadRequestError("Too many filters specified", this.filter);
      }
      if (this.filter.length === 1) {
        var newSegment = this.originalSegments.concat(this.filter[0]);
        this.url.segment(newSegment);
      }
    }
  }, {
    key: "_requestFnForLink",
    value: function _requestFnForLink(link) {
      var _this3 = this;
      return _asyncToGenerator(_regeneratorRuntime().mark(function _callee() {
        var opts,
          uri,
          template,
          r,
          _args = arguments;
        return _regeneratorRuntime().wrap(function _callee$(_context) {
          while (1) switch (_context.prev = _context.next) {
            case 0:
              opts = _args.length > 0 && _args[0] !== undefined ? _args[0] : {};
              if (link.templated) {
                template = (0, _URITemplate.default)(link.href);
                uri = (0, _urijs.default)(template.expand(opts));
              } else {
                uri = (0, _urijs.default)(link.href);
              }
              _context.next = 4;
              return _this3._sendNormalRequest(uri);
            case 4:
              r = _context.sent;
              return _context.abrupt("return", _this3._parseResponse(r));
            case 6:
            case "end":
              return _context.stop();
          }
        }, _callee);
      }));
    }
  }, {
    key: "_parseRecord",
    value: function _parseRecord(json) {
      var _this4 = this;
      if (!json._links) {
        return json;
      }
      Object.keys(json._links).forEach(function (key) {
        var n = json._links[key];
        var included = false;
        if (typeof json[key] !== "undefined") {
          json["".concat(key, "_attr")] = json[key];
          included = true;
        }
        if (included && JOINABLE.indexOf(key) >= 0) {
          var record = _this4._parseRecord(json[key]);
          json[key] = _asyncToGenerator(_regeneratorRuntime().mark(function _callee2() {
            return _regeneratorRuntime().wrap(function _callee2$(_context2) {
              while (1) switch (_context2.prev = _context2.next) {
                case 0:
                  return _context2.abrupt("return", record);
                case 1:
                case "end":
                  return _context2.stop();
              }
            }, _callee2);
          }));
        } else {
          json[key] = _this4._requestFnForLink(n);
        }
      });
      return json;
    }
  }, {
    key: "_sendNormalRequest",
    value: function () {
      var _sendNormalRequest2 = _asyncToGenerator(_regeneratorRuntime().mark(function _callee3(initialUrl) {
        var url;
        return _regeneratorRuntime().wrap(function _callee3$(_context3) {
          while (1) switch (_context3.prev = _context3.next) {
            case 0:
              url = initialUrl;
              if (url.authority() === "") {
                url = url.authority(this.url.authority());
              }
              if (url.protocol() === "") {
                url = url.protocol(this.url.protocol());
              }
              return _context3.abrupt("return", _horizon_axios_client.AxiosClient.get(url.toString()).then(function (response) {
                return response.data;
              }).catch(this._handleNetworkError));
            case 4:
            case "end":
              return _context3.stop();
          }
        }, _callee3, this);
      }));
      function _sendNormalRequest(_x) {
        return _sendNormalRequest2.apply(this, arguments);
      }
      return _sendNormalRequest;
    }()
  }, {
    key: "_parseResponse",
    value: function _parseResponse(json) {
      if (json._embedded && json._embedded.records) {
        return this._toCollectionPage(json);
      }
      return this._parseRecord(json);
    }
  }, {
    key: "_toCollectionPage",
    value: function _toCollectionPage(json) {
      var _this5 = this;
      for (var i = 0; i < json._embedded.records.length; i += 1) {
        json._embedded.records[i] = this._parseRecord(json._embedded.records[i]);
      }
      return {
        records: json._embedded.records,
        next: function () {
          var _next2 = _asyncToGenerator(_regeneratorRuntime().mark(function _callee4() {
            var r;
            return _regeneratorRuntime().wrap(function _callee4$(_context4) {
              while (1) switch (_context4.prev = _context4.next) {
                case 0:
                  _context4.next = 2;
                  return _this5._sendNormalRequest((0, _urijs.default)(json._links.next.href));
                case 2:
                  r = _context4.sent;
                  return _context4.abrupt("return", _this5._toCollectionPage(r));
                case 4:
                case "end":
                  return _context4.stop();
              }
            }, _callee4);
          }));
          function next() {
            return _next2.apply(this, arguments);
          }
          return next;
        }(),
        prev: function () {
          var _prev = _asyncToGenerator(_regeneratorRuntime().mark(function _callee5() {
            var r;
            return _regeneratorRuntime().wrap(function _callee5$(_context5) {
              while (1) switch (_context5.prev = _context5.next) {
                case 0:
                  _context5.next = 2;
                  return _this5._sendNormalRequest((0, _urijs.default)(json._links.prev.href));
                case 2:
                  r = _context5.sent;
                  return _context5.abrupt("return", _this5._toCollectionPage(r));
                case 4:
                case "end":
                  return _context5.stop();
              }
            }, _callee5);
          }));
          function prev() {
            return _prev.apply(this, arguments);
          }
          return prev;
        }()
      };
    }
  }, {
    key: "_handleNetworkError",
    value: (function () {
      var _handleNetworkError2 = _asyncToGenerator(_regeneratorRuntime().mark(function _callee6(error) {
        var _error$response$statu, _error$response$statu2;
        return _regeneratorRuntime().wrap(function _callee6$(_context6) {
          while (1) switch (_context6.prev = _context6.next) {
            case 0:
              if (!(error.response && error.response.status)) {
                _context6.next = 8;
                break;
              }
              _context6.t0 = error.response.status;
              _context6.next = _context6.t0 === 404 ? 4 : 5;
              break;
            case 4:
              return _context6.abrupt("return", Promise.reject(new _errors.NotFoundError((_error$response$statu = error.response.statusText) !== null && _error$response$statu !== void 0 ? _error$response$statu : "Not Found", error.response.data)));
            case 5:
              return _context6.abrupt("return", Promise.reject(new _errors.NetworkError((_error$response$statu2 = error.response.statusText) !== null && _error$response$statu2 !== void 0 ? _error$response$statu2 : "Unknown", error.response.data)));
            case 6:
              _context6.next = 9;
              break;
            case 8:
              return _context6.abrupt("return", Promise.reject(new Error(error.message)));
            case 9:
            case "end":
              return _context6.stop();
          }
        }, _callee6);
      }));
      function _handleNetworkError(_x2) {
        return _handleNetworkError2.apply(this, arguments);
      }
      return _handleNetworkError;
    }())
  }]);
}();