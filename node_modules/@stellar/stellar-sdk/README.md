
<div align="center">
  <img alt="Stellar" src="https://github.com/stellar/.github/raw/master/stellar-logo.png" width="558" />
  <br/>
  <strong>Creating equitable access to the global financial system</strong>
  <h1>js-stellar-sdk</h1>
</div>

<p align="center">
  <a href="https://badge.fury.io/js/@stellar%2Fstellar-sdk"><img src="https://badge.fury.io/js/@stellar%2Fstellar-sdk.svg" alt="npm version" height="18"></a>
  <a href="https://www.npmjs.com/package/@stellar/stellar-sdk">
    <img alt="Weekly Downloads" src="https://img.shields.io/npm/dw/@stellar/stellar-sdk" />
  </a>
  <a href="https://github.com/stellar/js-stellar-sdk/actions/workflows/tests.yml"><img alt="Test Status" src="https://github.com/stellar/js-stellar-sdk/actions/workflows/tests.yml/badge.svg" /></a>
</p>

js-stellar-sdk is a JavaScript library for communicating with a
[Stellar Horizon server](https://github.com/stellar/go/tree/master/services/horizon) and [Soroban RPC](https://developers.stellar.org/docs/data/rpc).
It is used for building Stellar apps either on Node.js or in the browser, though it can be used in other environments with some tinkering.

It provides:

- a networking layer API for Horizon endpoints (REST-based),
- a networking layer for Soroban RPC (JSONRPC-based).
- facilities for building and signing transactions, for communicating with a
  Stellar Horizon instance, and for submitting transactions or querying network
  history.

**Jump to:**

 * [Installation](#installation): details on hitting the ground running
 * [Usage](#usage): links to documentation and a variety of workarounds for non-traditional JavaScript environments
   - [...with React Native](#usage-with-react-native)
   - [...with Expo](#usage-with-expo-managed-workflows)
   - [...with CloudFlare Workers](#usage-with-cloudflare-workers)
 * [Developing](#developing): contribute to the project!
 * [Understanding `stellar-sdk` vs. `stellar-base`](#stellar-sdk-vs-stellar-base)
 * [License](#license)

## Installation

Using npm or yarn to include `stellar-sdk` in your own project:

```shell
npm install --save @stellar/stellar-sdk
# or
yarn add @stellar/stellar-sdk
```

Then, require or import it in your JavaScript code:

```js
var StellarSdk = require('@stellar/stellar-sdk');
// or
import * as StellarSdk from '@stellar/stellar-sdk';
```

(Preferably, you would only import the pieces you need to enable tree-shaking and lower your final bundle sizes.)

### Browsers

You can use a CDN:

```html
<script src="https://cdnjs.cloudflare.com/ajax/libs/stellar-sdk/{version}/stellar-sdk.js"></script>
```

Note that this method relies on using a third party to host the JS library. This may not be entirely secure. You can self-host it via [Bower](http://bower.io):

```shell
bower install @stellar/stellar-sdk
```

and include it in the browser:

```html
<script src="./bower_components/stellar-sdk/stellar-sdk.js"></script>
<script>
  console.log(StellarSdk);
</script>
```

If you don't want to use or install Bower, you can copy the packaged JS files from the [Bower repo](https://github.com/stellar/bower-js-stellar-sdk), or just build the package yourself locally (see [Developing :arrow_right: Building](#building)) and copy the bundle.

| Always make sure that you are using the latest version number. They can be found on the [releases page](https://github.com/stellar/js-stellar-sdk/releases) in GitHub. |
|----|

### Custom Installation

You can configure whether or not to build the browser bundle with the axios dependency. In order to turn off the axios dependency, set the USE_AXIOS environment variable to false. You can also turn off the eventsource dependency by setting USE_EVENTSOURCE to false.

#### Build without Axios
```
npm run build:browser:no-axios
```
This will create `stellar-sdk-no-axios.js` in `dist/`.

#### Build without EventSource
```
npm run build:browser:no-eventsource
```
This will create `stellar-sdk-no-eventsource.js` in `dist/`.

#### Build without Axios and Eventsource
```
npm run build:browser:minimal
```
This will create `stellar-sdk-minimal.js` in `dist/`.

## Usage

The usage documentation for this library lives in a handful of places:

 * across the [Stellar Developer Docs](https://developers.stellar.org), which includes tutorials and examples,
 * within [this repository itself](https://github.com/stellar/js-stellar-sdk/blob/master/docs/reference/readme.md), and
 * on the generated [API doc site](https://stellar.github.io/js-stellar-sdk/).

You can also refer to:

 * the [documentation](https://developers.stellar.org/docs/data/horizon) for the Horizon REST API (if using the `Horizon` module) and
 * the [documentation](https://developers.stellar.org/docs/data/rpc) for Soroban RPC's API (if using the `rpc` module)

### Usage with React-Native

1. Install `yarn add --dev rn-nodeify`
2. Add the following postinstall script:
```
yarn rn-nodeify --install url,events,https,http,util,stream,crypto,vm,buffer --hack --yarn
```
3. Uncomment `require('crypto')` on shim.js
4. `react-native link react-native-randombytes`
5. Create file `rn-cli.config.js`
```
module.exports = {
  resolver: {
    extraNodeModules: require("node-libs-react-native"),
  },
};
```
6. Add `import "./shim";` to the top of `index.js`
7. `yarn add @stellar/stellar-sdk`

There is also a [sample](https://github.com/fnando/rn-stellar-sdk-sample) that you can follow.

**Note**: Only the V8 compiler (on Android) and JSC (on iOS) have proper support for `Buffer` and `Uint8Array` as is needed by this library. Otherwise, you may see bizarre errors when doing XDR encoding/decoding such as `source not specified`.

#### Usage with Expo managed workflows

1. Install `yarn add --dev rn-nodeify`
2. Add the following postinstall script:
```
yarn rn-nodeify --install process,url,events,https,http,util,stream,crypto,vm,buffer --hack --yarn
```
3. Add `import "./shim";` to your app's entry point (by default `./App.js`)
4. `yarn add @stellar/stellar-sdk`
5. `expo install expo-random`

At this point, the Stellar SDK will work, except that `StellarSdk.Keypair.random()` will throw an error. To work around this, you can create your own method to generate a random keypair like this:

```javascript
import * as Random from 'expo-random';
import { Keypair } from '@stellar/stellar-sdk';

const generateRandomKeypair = () => {
  const randomBytes = Random.getRandomBytes(32);
  return Keypair.fromRawEd25519Seed(Buffer.from(randomBytes));
};
```

#### Usage with CloudFlare Workers

Both `eventsource` (needed for streaming) and `axios` (needed for making HTTP requests) are problematic dependencies in the CFW environment. The experimental branch [`make-eventsource-optional`](https://github.com/stellar/js-stellar-sdk/pull/901) is an attempt to resolve these issues.

It requires the following additional tweaks to your project:
 * the `axios-fetch-adapter` lets you use `axios` with `fetch` as a backend, which is available to CF workers
 * it only works with `axios@"<= 1.0.0"` versions, so we need to force an override into the underlying dependency
 * and this can be problematic with newer `yarn` versions, so we need to force the environment to use Yarn 1

In summary, the `package.json` tweaks look something like this:

```jsonc
"dependencies": {
  // ...
  "@stellar/stellar-sdk": "git+https://github.com/stellar/js-stellar-sdk#make-eventsource-optional",
  "@vespaiach/axios-fetch-adapter": "^0.3.1",
  "axios": "^0.26.1"
},
"overrides": {
  "@stellar/stellar-sdk": {
    "axios": "$axios"
  }
},
"packageManager": "yarn@1.22.19"
```

Then, you need to override the adapter in your codebase:

```typescript
import { Horizon } from '@stellar/stellar-sdk';
import fetchAdapter from '@vespaiach/axios-fetch-adapter';

Horizon.AxiosClient.defaults.adapter = fetchAdapter as any;

// then, the rest of your code...
```

All HTTP calls will use `fetch`, now, meaning it should work in the CloudFlare Worker environment.

## Developing

So you want to contribute to the library: welcome! Whether you're working on a fork or want to make an upstream request, the dev-test loop is pretty straightforward.

1. Clone the repo:

```shell
git clone https://github.com/stellar/js-stellar-sdk.git
```

2. Install dependencies inside js-stellar-sdk folder:

```shell
cd js-stellar-sdk
yarn
```

3. Install Node 18

Because we support the oldest maintenance version of Node, please install and develop on Node 18 so you don't get surprised when your code works locally but breaks in CI.

Here's how to install `nvm` if you haven't: https://github.com/creationix/nvm

```shell
nvm install 18

# if you've never installed 18 before you'll want to re-install yarn
npm install -g yarn
```

If you work on several projects that use different Node versions, you might it helpful to install this automatic version manager: https://github.com/wbyoung/avn

4. Observe the project's code style

While you're making changes, make sure to run the linter to catch any linting
errors (in addition to making sure your text editor supports ESLint) and conform to the project's code style.

```shell
yarn fmt
```

### Building
You can build the developer version (unoptimized, commented, with source maps, etc.) or the production bundles:

```shell
yarn build
# or
yarn build:prod
```

### Testing

To run all tests:

```shell
yarn test
```

To run a specific set of tests:

```shell
yarn test:node
yarn test:browser
yarn test:integration
```

In order to have a faster test loop, these suite-specific commands **do not** build the bundles first (unlike `yarn test`). If you make code changes, you will need to run `yarn build` (or a subset like `yarn build:node` corresponding to the test suite) before running the tests again to see your changes.

To generate and check the documentation site:

```shell
# install the `serve` command if you don't have it already
npm i -g serve

# clone the base library for complete docs
git clone https://github.com/stellar/js-stellar-base

# generate the docs files
yarn docs

# get these files working in a browser
cd jsdoc && serve .

# you'll be able to browse the docs at http://localhost:5000
```

### Publishing

For information on how to contribute or publish new versions of this software to `npm`, please refer to our [contribution guide](https://github.com/stellar/js-stellar-sdk/blob/master/CONTRIBUTING.md).

## Miscellaneous

### `stellar-sdk` vs `stellar-base`

`stellar-sdk` is a high-level library that serves as client-side API for Horizon and Soroban RPC, while [`stellar-base](https://github.com/stellar/js-stellar-base) is lower-level library for creating Stellar primitive constructs via XDR helpers and wrappers.

**Most people will want stellar-sdk instead of stellar-base.** You should only use stellar-base if you know what you're doing!

If you add `stellar-sdk` to a project, **do not add `stellar-base`!** Mismatching versions could cause weird, hard-to-find bugs. `stellar-sdk` automatically installs `stellar-base` and exposes all of its exports in case you need them.

> **Important!** The Node.js version of the `stellar-base` (`stellar-sdk` dependency) package uses the [`sodium-native`](https://www.npmjs.com/package/sodium-native) package as an [optional dependency](https://docs.npmjs.com/files/package.json#optionaldependencies). `sodium-native` is a low level binding to [libsodium](https://github.com/jedisct1/libsodium), (an implementation of [Ed25519](https://ed25519.cr.yp.to/) signatures).
> If installation of `sodium-native` fails, or it is unavailable, `stellar-base` (and `stellar-sdk`) will fallback to using the [`tweetnacl`](https://www.npmjs.com/package/tweetnacl) package implementation. If you are using them in a browser, you can ignore this. However, for production backend deployments, you should be using `sodium-native`.
> If `sodium-native` is successfully installed and working the `StellarSdk.FastSigning` variable will return `true`.

### License

js-stellar-sdk is licensed under an Apache-2.0 license. See the
[LICENSE](https://github.com/stellar/js-stellar-sdk/blob/master/LICENSE) file
for details.
