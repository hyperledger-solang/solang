'use strict';

Object.defineProperty(exports, '__esModule', { value: true });

var isRetryAllowed = require('is-retry-allowed');

function _interopDefault (e) { return e && e.__esModule ? e : { default: e }; }

var isRetryAllowed__default = /*#__PURE__*/_interopDefault(isRetryAllowed);

// src/retry.ts
function isNetworkError(error) {
  const CODE_EXCLUDE_LIST = ["ERR_CANCELED", "ECONNABORTED"];
  if (error.response) {
    return false;
  }
  if (!error.code) {
    return false;
  }
  if (CODE_EXCLUDE_LIST.includes(error.code)) {
    return false;
  }
  return isRetryAllowed__default.default(error);
}
var SAFE_HTTP_METHODS = ["get", "head", "options"];
var IDEMPOTENT_HTTP_METHODS = SAFE_HTTP_METHODS.concat(["put", "delete"]);
function isRetryableError(error) {
  return error.code !== "ECONNABORTED" && (!error.response || error.response.status >= 500 && error.response.status <= 599);
}
function isSafeRequestError(error) {
  if (!error.config?.method) {
    return false;
  }
  return isRetryableError(error) && SAFE_HTTP_METHODS.indexOf(error.config.method) !== -1;
}
function isIdempotentRequestError(error) {
  if (!error.config?.method) {
    return false;
  }
  return isRetryableError(error) && IDEMPOTENT_HTTP_METHODS.indexOf(error.config.method) !== -1;
}
function isNetworkOrIdempotentRequestError(error) {
  return isNetworkError(error) || isIdempotentRequestError(error);
}
function noDelay() {
  return 0;
}
function exponentialDelay(retryNumber = 0, _error = void 0, delayFactor = 100) {
  const delay = 2 ** retryNumber * delayFactor;
  const randomSum = delay * 0.2 * Math.random();
  return delay + randomSum;
}
var DEFAULT_OPTIONS = {
  retries: 3,
  retryCondition: isNetworkOrIdempotentRequestError,
  retryDelay: noDelay,
  shouldResetTimeout: false,
  onRetry: () => {
  }
};
function getRequestOptions(config, defaultOptions) {
  return { ...DEFAULT_OPTIONS, ...defaultOptions, ...config.retry };
}
function setCurrentState(config, defaultOptions) {
  const currentState = getRequestOptions(config, defaultOptions || {});
  currentState.retryCount = currentState.retryCount || 0;
  currentState.lastRequestTime = currentState.lastRequestTime || Date.now();
  config.retry = currentState;
  return currentState;
}
async function shouldRetry(currentState, error) {
  const { retries, retryCondition } = currentState;
  const shouldRetryOrPromise = (currentState.retryCount || 0) < retries && retryCondition(error);
  if (typeof shouldRetryOrPromise === "object") {
    try {
      const shouldRetryPromiseResult = await shouldRetryOrPromise;
      return shouldRetryPromiseResult !== false;
    } catch (_err) {
      return false;
    }
  }
  return shouldRetryOrPromise;
}
var axiosRetry = (axiosInstance, defaultOptions) => {
  const requestInterceptorId = axiosInstance.interceptors.request.use(
    (config) => {
      setCurrentState(config, defaultOptions);
      return config;
    }
  );
  const responseInterceptorId = axiosInstance.interceptors.response.use(
    null,
    async (error) => {
      const { config } = error;
      if (!config) {
        return Promise.reject(error);
      }
      const currentState = setCurrentState(config, defaultOptions);
      if (await shouldRetry(currentState, error)) {
        currentState.retryCount += 1;
        const { retryDelay, shouldResetTimeout, onRetry } = currentState;
        const delay = retryDelay(currentState.retryCount, error);
        if (!shouldResetTimeout && config.timeout && currentState.lastRequestTime) {
          const lastRequestDuration = Date.now() - currentState.lastRequestTime;
          const timeout = config.timeout - lastRequestDuration - delay;
          if (timeout <= 0) {
            return Promise.reject(error);
          }
          config.timeout = timeout;
        }
        config.transformRequest = [(data) => data];
        await onRetry(currentState.retryCount, error, config);
        return new Promise((resolve) => {
          setTimeout(() => resolve(axiosInstance(config)), delay);
        });
      }
      return Promise.reject(error);
    }
  );
  return { requestInterceptorId, responseInterceptorId };
};
axiosRetry.isNetworkError = isNetworkError;
axiosRetry.isSafeRequestError = isSafeRequestError;
axiosRetry.isIdempotentRequestError = isIdempotentRequestError;
axiosRetry.isNetworkOrIdempotentRequestError = isNetworkOrIdempotentRequestError;
axiosRetry.exponentialDelay = exponentialDelay;
axiosRetry.isRetryableError = isRetryableError;
var retry_default = axiosRetry;

exports.DEFAULT_OPTIONS = DEFAULT_OPTIONS;
exports.default = retry_default;
exports.exponentialDelay = exponentialDelay;
exports.isIdempotentRequestError = isIdempotentRequestError;
exports.isNetworkError = isNetworkError;
exports.isNetworkOrIdempotentRequestError = isNetworkOrIdempotentRequestError;
exports.isRetryableError = isRetryableError;
exports.isSafeRequestError = isSafeRequestError;
