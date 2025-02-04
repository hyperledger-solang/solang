"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.create = createFetchClient;
exports.fetchClient = void 0;
var _feaxios = _interopRequireDefault(require("feaxios"));
var _types = require("./types");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError("Cannot call a class as a function"); }
function _defineProperties(e, r) { for (var t = 0; t < r.length; t++) { var o = r[t]; o.enumerable = o.enumerable || !1, o.configurable = !0, "value" in o && (o.writable = !0), Object.defineProperty(e, _toPropertyKey(o.key), o); } }
function _createClass(e, r, t) { return r && _defineProperties(e.prototype, r), t && _defineProperties(e, t), Object.defineProperty(e, "prototype", { writable: !1 }), e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
var InterceptorManager = function () {
  function InterceptorManager() {
    _classCallCheck(this, InterceptorManager);
    _defineProperty(this, "handlers", []);
  }
  return _createClass(InterceptorManager, [{
    key: "use",
    value: function use(fulfilled, rejected) {
      this.handlers.push({
        fulfilled: fulfilled,
        rejected: rejected
      });
      return this.handlers.length - 1;
    }
  }, {
    key: "eject",
    value: function eject(id) {
      if (this.handlers[id]) {
        this.handlers[id] = null;
      }
    }
  }, {
    key: "forEach",
    value: function forEach(fn) {
      this.handlers.forEach(function (h) {
        if (h !== null) {
          fn(h);
        }
      });
    }
  }]);
}();
function getFormConfig(config) {
  var formConfig = config || {};
  formConfig.headers = new Headers(formConfig.headers || {});
  formConfig.headers.set('Content-Type', 'application/x-www-form-urlencoded');
  return formConfig;
}
function createFetchClient() {
  var fetchConfig = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  var defaults = _objectSpread(_objectSpread({}, fetchConfig), {}, {
    headers: fetchConfig.headers || {}
  });
  var instance = _feaxios.default.create(defaults);
  var requestInterceptors = new InterceptorManager();
  var responseInterceptors = new InterceptorManager();
  var httpClient = {
    interceptors: {
      request: requestInterceptors,
      response: responseInterceptors
    },
    defaults: _objectSpread(_objectSpread({}, defaults), {}, {
      adapter: function adapter(config) {
        return instance.request(config);
      }
    }),
    create: function create(config) {
      return createFetchClient(_objectSpread(_objectSpread({}, this.defaults), config));
    },
    makeRequest: function makeRequest(config) {
      var _this = this;
      return new Promise(function (resolve, reject) {
        var abortController = new AbortController();
        config.signal = abortController.signal;
        if (config.cancelToken) {
          config.cancelToken.promise.then(function () {
            abortController.abort();
            reject(new Error('Request canceled'));
          });
        }
        var modifiedConfig = config;
        if (requestInterceptors.handlers.length > 0) {
          var chain = requestInterceptors.handlers.filter(function (interceptor) {
            return interceptor !== null;
          }).flatMap(function (interceptor) {
            return [interceptor.fulfilled, interceptor.rejected];
          });
          for (var i = 0, len = chain.length; i < len; i += 2) {
            var onFulfilled = chain[i];
            var onRejected = chain[i + 1];
            try {
              if (onFulfilled) modifiedConfig = onFulfilled(modifiedConfig);
            } catch (error) {
              if (onRejected) onRejected === null || onRejected === void 0 || onRejected(error);
              reject(error);
              return;
            }
          }
        }
        var adapter = modifiedConfig.adapter || _this.defaults.adapter;
        if (!adapter) {
          throw new Error('No adapter available');
        }
        var responsePromise = adapter(modifiedConfig).then(function (axiosResponse) {
          var httpClientResponse = {
            data: axiosResponse.data,
            headers: axiosResponse.headers,
            config: axiosResponse.config,
            status: axiosResponse.status,
            statusText: axiosResponse.statusText
          };
          return httpClientResponse;
        });
        if (responseInterceptors.handlers.length > 0) {
          var _chain = responseInterceptors.handlers.filter(function (interceptor) {
            return interceptor !== null;
          }).flatMap(function (interceptor) {
            return [interceptor.fulfilled, interceptor.rejected];
          });
          var _loop = function _loop(_i) {
            responsePromise = responsePromise.then(function (response) {
              var fulfilledInterceptor = _chain[_i];
              if (typeof fulfilledInterceptor === 'function') {
                return fulfilledInterceptor(response);
              }
              return response;
            }, function (error) {
              var rejectedInterceptor = _chain[_i + 1];
              if (typeof rejectedInterceptor === 'function') {
                return rejectedInterceptor(error);
              }
              throw error;
            }).then(function (interceptedResponse) {
              return interceptedResponse;
            });
          };
          for (var _i = 0, _len = _chain.length; _i < _len; _i += 2) {
            _loop(_i);
          }
        }
        responsePromise.then(resolve).catch(reject);
      });
    },
    get: function get(url, config) {
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), config), {}, {
        url: url,
        method: 'get'
      }));
    },
    delete: function _delete(url, config) {
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), config), {}, {
        url: url,
        method: 'delete'
      }));
    },
    head: function head(url, config) {
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), config), {}, {
        url: url,
        method: 'head'
      }));
    },
    options: function options(url, config) {
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), config), {}, {
        url: url,
        method: 'options'
      }));
    },
    post: function post(url, data, config) {
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), config), {}, {
        url: url,
        method: 'post',
        data: data
      }));
    },
    put: function put(url, data, config) {
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), config), {}, {
        url: url,
        method: 'put',
        data: data
      }));
    },
    patch: function patch(url, data, config) {
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), config), {}, {
        url: url,
        method: 'patch',
        data: data
      }));
    },
    postForm: function postForm(url, data, config) {
      var formConfig = getFormConfig(config);
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), formConfig), {}, {
        url: url,
        method: 'post',
        data: data
      }));
    },
    putForm: function putForm(url, data, config) {
      var formConfig = getFormConfig(config);
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), formConfig), {}, {
        url: url,
        method: 'put',
        data: data
      }));
    },
    patchForm: function patchForm(url, data, config) {
      var formConfig = getFormConfig(config);
      return this.makeRequest(_objectSpread(_objectSpread(_objectSpread({}, this.defaults), formConfig), {}, {
        url: url,
        method: 'patch',
        data: data
      }));
    },
    CancelToken: _types.CancelToken,
    isCancel: function isCancel(value) {
      return value instanceof Error && value.message === 'Request canceled';
    }
  };
  return httpClient;
}
var fetchClient = exports.fetchClient = createFetchClient();