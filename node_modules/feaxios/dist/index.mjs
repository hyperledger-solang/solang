// src/client.ts
async function prepareAxiosResponse(options, res) {
  const response = { config: options };
  response.status = res.status;
  response.statusText = res.statusText;
  response.headers = res.headers;
  if (options.responseType === "stream") {
    response.data = res.body;
    return response;
  }
  return res[options.responseType || "text"]().then((data) => {
    if (options.transformResponse) {
      Array.isArray(options.transformResponse) ? options.transformResponse.map(
        (fn) => data = fn.call(options, data, res?.headers, res?.status)
      ) : data = options.transformResponse(data, res?.headers, res?.status);
      response.data = data;
    } else {
      response.data = data;
      response.data = JSON.parse(data);
    }
  }).catch(Object).then(() => response);
}
async function handleFetch(options, fetchOptions) {
  let res = null;
  if ("any" in AbortSignal) {
    const signals = [];
    if (options.timeout) {
      signals.push(AbortSignal.timeout(options.timeout));
    }
    if (options.signal) {
      signals.push(options.signal);
    }
    if (signals.length > 0) {
      fetchOptions.signal = AbortSignal.any(signals);
    }
  } else {
    if (options.timeout) {
      fetchOptions.signal = AbortSignal.timeout(options.timeout);
    }
  }
  try {
    res = await fetch(options.url, fetchOptions);
    const ok = options.validateStatus ? options.validateStatus(res.status) : res.ok;
    if (!ok) {
      return Promise.reject(
        new AxiosError(
          `Request failed with status code ${res?.status}`,
          [AxiosError.ERR_BAD_REQUEST, AxiosError.ERR_BAD_RESPONSE][Math.floor(res?.status / 100) - 4],
          options,
          new Request(options.url, fetchOptions),
          await prepareAxiosResponse(options, res)
        )
      );
    }
    return await prepareAxiosResponse(options, res);
  } catch (error) {
    if (error.name === "AbortError" || error.name === "TimeoutError") {
      const isTimeoutError = error.name === "TimeoutError";
      return Promise.reject(
        isTimeoutError ? new AxiosError(
          options.timeoutErrorMessage || `timeout of ${options.timeout} ms exceeded`,
          AxiosError.ECONNABORTED,
          options,
          request
        ) : new CanceledError(null, options)
      );
    }
    return Promise.reject(
      new AxiosError(
        error.message,
        void 0,
        options,
        request,
        void 0
      )
    );
  }
}
function buildURL(options) {
  let url = options.url || "";
  if (options.baseURL && options.url) {
    url = options.url.replace(/^(?!.*\/\/)\/?/, `${options.baseURL}/`);
  }
  if (options.params && Object.keys(options.params).length > 0 && options.url) {
    url += (~options.url.indexOf("?") ? "&" : "?") + (options.paramsSerializer ? options.paramsSerializer(options.params) : new URLSearchParams(options.params));
  }
  return url;
}
function mergeAxiosOptions(input, defaults) {
  const merged = {
    ...defaults,
    ...input
  };
  if (defaults?.params && input?.params) {
    merged.params = {
      ...defaults?.params,
      ...input?.params
    };
  }
  if (defaults?.headers && input?.headers) {
    merged.headers = new Headers(defaults.headers || {});
    const headers = new Headers(input.headers || {});
    headers.forEach((value, key) => {
      merged.headers.set(key, value);
    });
  }
  return merged;
}
function mergeFetchOptions(input, defaults) {
  const merged = {
    ...defaults,
    ...input
  };
  if (defaults?.headers && input?.headers) {
    merged.headers = new Headers(defaults.headers || {});
    const headers = new Headers(input.headers || {});
    headers.forEach((value, key) => {
      merged.headers.set(key, value);
    });
  }
  return merged;
}
function defaultTransformer(data, headers) {
  const contentType = headers.get("content-type");
  if (!contentType) {
    if (typeof data === "string") {
      headers.set("content-type", "text/plain");
    } else if (data instanceof URLSearchParams) {
      headers.set("content-type", "application/x-www-form-urlencoded");
    } else if (data instanceof Blob || data instanceof ArrayBuffer || ArrayBuffer.isView(data)) {
      headers.set("content-type", "application/octet-stream");
    } else if (typeof data === "object" && typeof data.append !== "function" && typeof data.text !== "function") {
      data = JSON.stringify(data);
      headers.set("content-type", "application/json");
    }
  } else {
    if (contentType === "application/x-www-form-urlencoded" && !(data instanceof URLSearchParams)) {
      data = new URLSearchParams(data);
    } else if (contentType === "application/json" && typeof data === "object") {
      data = JSON.stringify(data);
    }
  }
  return data;
}
async function request(configOrUrl, config, defaults, method, interceptors, data) {
  if (typeof configOrUrl === "string") {
    config = config || {};
    config.url = configOrUrl;
  } else
    config = configOrUrl || {};
  const options = mergeAxiosOptions(config, defaults || {});
  options.fetchOptions = options.fetchOptions || {};
  options.timeout = options.timeout || 0;
  options.headers = new Headers(options.headers || {});
  options.transformRequest = options.transformRequest ?? defaultTransformer;
  data = data || options.data;
  if (options.transformRequest && data) {
    Array.isArray(options.transformRequest) ? options.transformRequest.map(
      (fn) => data = fn.call(options, data, options.headers)
    ) : data = options.transformRequest(data, options.headers);
  }
  options.url = buildURL(options);
  options.method = method || options.method || "get";
  if (interceptors && interceptors.request.handlers.length > 0) {
    const chain = interceptors.request.handlers.filter(
      (interceptor) => !interceptor?.runWhen || typeof interceptor.runWhen === "function" && interceptor.runWhen(options)
    ).flatMap((interceptor) => [interceptor.fulfilled, interceptor.rejected]);
    let result = options;
    for (let i = 0, len = chain.length; i < len; i += 2) {
      const onFulfilled = chain[i];
      const onRejected = chain[i + 1];
      try {
        if (onFulfilled)
          result = onFulfilled(result);
      } catch (error) {
        if (onRejected)
          onRejected?.(error);
        break;
      }
    }
  }
  const init = mergeFetchOptions(
    {
      method: options.method?.toUpperCase(),
      body: data,
      headers: options.headers,
      credentials: options.withCredentials ? "include" : void 0,
      signal: options.signal
    },
    options.fetchOptions
  );
  let resp = handleFetch(options, init);
  if (interceptors && interceptors.response.handlers.length > 0) {
    const chain = interceptors.response.handlers.flatMap((interceptor) => [
      interceptor.fulfilled,
      interceptor.rejected
    ]);
    for (let i = 0, len = chain.length; i < len; i += 2) {
      resp = resp.then(chain[i], chain[i + 1]);
    }
  }
  return resp;
}
var AxiosInterceptorManager = class {
  handlers = [];
  constructor() {
    this.handlers = [];
  }
  use = (onFulfilled, onRejected, options) => {
    this.handlers.push({
      fulfilled: onFulfilled,
      rejected: onRejected,
      runWhen: options?.runWhen
    });
    return this.handlers.length - 1;
  };
  eject = (id) => {
    if (this.handlers[id]) {
      this.handlers[id] = null;
    }
  };
  clear = () => {
    this.handlers = [];
  };
};
function createAxiosInstance(defaults) {
  defaults = defaults || {};
  const interceptors = {
    request: new AxiosInterceptorManager(),
    response: new AxiosInterceptorManager()
  };
  const axios2 = (url, config) => request(url, config, defaults, void 0, interceptors);
  axios2.defaults = defaults;
  axios2.interceptors = interceptors;
  axios2.getUri = (config) => {
    const merged = mergeAxiosOptions(config || {}, defaults);
    return buildURL(merged);
  };
  axios2.request = (config) => request(config, void 0, defaults, void 0, interceptors);
  ["get", "delete", "head", "options"].forEach((method) => {
    axios2[method] = (url, config) => request(url, config, defaults, method, interceptors);
  });
  ["post", "put", "patch"].forEach((method) => {
    axios2[method] = (url, data, config) => request(url, config, defaults, method, interceptors, data);
  });
  ["postForm", "putForm", "patchForm"].forEach((method) => {
    axios2[method] = (url, data, config) => {
      config = config || {};
      config.headers = new Headers(config.headers || {});
      config.headers.set("content-type", "application/x-www-form-urlencoded");
      return request(
        url,
        config,
        defaults,
        method.replace("Form", ""),
        interceptors,
        data
      );
    };
  });
  return axios2;
}
var AxiosError = class extends Error {
  config;
  code;
  request;
  response;
  status;
  isAxiosError;
  constructor(message, code, config, request2, response) {
    super(message);
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    } else {
      this.stack = new Error().stack;
    }
    this.name = "AxiosError";
    this.code = code;
    this.config = config;
    this.request = request2;
    this.response = response;
    this.isAxiosError = true;
  }
  static ERR_BAD_OPTION_VALUE = "ERR_BAD_OPTION_VALUE";
  static ERR_BAD_OPTION = "ERR_BAD_OPTION";
  static ERR_NETWORK = "ERR_NETWORK";
  static ERR_BAD_RESPONSE = "ERR_BAD_RESPONSE";
  static ERR_BAD_REQUEST = "ERR_BAD_REQUEST";
  static ERR_INVALID_URL = "ERR_INVALID_URL";
  static ERR_CANCELED = "ERR_CANCELED";
  static ECONNABORTED = "ECONNABORTED";
  static ETIMEDOUT = "ETIMEDOUT";
};
var CanceledError = class extends AxiosError {
  constructor(message, config, request2) {
    super(
      !message ? "canceled" : message,
      AxiosError.ERR_CANCELED,
      config,
      request2
    );
    this.name = "CanceledError";
  }
};
function isAxiosError(payload) {
  return payload !== null && typeof payload === "object" && payload.isAxiosError;
}
var axios = createAxiosInstance();
axios.create = (defaults) => createAxiosInstance(defaults);

// src/index.ts
var src_default = axios;

export { AxiosError, CanceledError, src_default as default, isAxiosError };
