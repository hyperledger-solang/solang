# feaxios

`feaxios` is a lightweight alternative to **Axios**, providing the same familiar API with a significantly reduced footprint of **2KB**. It leverages the native `fetch()` API supported in all modern browsers, delivering a performant and minimalistic solution. This makes it an ideal choice for projects where minimizing bundle size is a priority.

### Key Features

- **Lightweight:** With a size of less than 1/5th of Axios, `feaxios` is an efficient choice for projects with strict size constraints.

- **Native Fetch API:** Utilizes the browser's native fetch, ensuring broad compatibility and seamless integration with modern web development tools.

- **Interceptor Support:** `feaxios` supports interceptors, allowing you to customize and augment the request and response handling process.

- **Timeouts:** Easily configure timeouts for requests, ensuring your application remains responsive and resilient.

- **Retries:** Axios retry package is integrated with feaxios.


### When to Use feaxios

While [Axios] remains an excellent module, `feaxios` provides a compelling option in scenarios where minimizing dependencies is crucial. By offering a similar API to Axios, `feaxios` bridges the gap between Axios and the native `fetch()` API.

```sh
npm install feaxios
```

**_Request Config_**

```ts
{

url: '/user',

method: 'get', // default

baseURL: 'https://some-domain.com/api/',

transformRequest: [function (data, headers) {
  return data;
}],

transformResponse: [function (data) {

    return data;
}],

headers: {'test': 'test'},

params: {
    ID: 12345
},

 paramsSerializer: {

    encode?: (param: string): string => {},

    serialize?: (params: Record<string, any>, options?: ParamsSerializerOptions ),

    indexes: false
  },

  data: {},

  timeout: 1000, // default is 0ms

  withCredentials: false,

  responseType: 'json', // default

  validateStatus: function (status) {
    return status >= 200 && status < 300;
  },

  signal: new AbortController().signal,

  fetchOptions:  {
     redirect: "follow"
  },
 retry: { retries: 3 }
```

**In fetchOptions you can pass custom options like proxy , agents etc supported on nodejs**

### Usage

```js
import axios from "feaxios";

axios
  .get("https://api.example.com/data")
  .then((response) => {
    // Handle the response
    console.log(response.data);
  })
  .catch((error) => {
    // Handle errors
    console.error(error);
  });
```

**_With Interceptors_**

```js
import axios from "feaxios";

axios.interceptors.request.use((config) => {
  config.headers.set("Authorization", "Bearer *");
  return config;
});
axios.interceptors.response.use(
  function (response) {
    return response;
  },
  function (error) {
    //do something with error
    return Promise.reject(error);
  },
);
```
**Axios Retry Package is also ported to feaxios**

```ts
import axios from "feaxios"
import axiosRetry from "feaxios/retry"

const http = axios.create({
  timeout: 3 * 1000 * 60,
})

axiosRetry(http, { retryDelay: axiosRetry.exponentialDelay })
```
Visit: https://github.com/softonic/axios-retry to see more options.
