
# Changelog

A breaking change will get clearly marked in this log.


## Unreleased


## [v13.1.0](https://github.com/stellar/js-stellar-sdk/compare/v13.0.0...v13.1.0)

### Added
* Added `Horizon.Server.root` to obtain information from the Horizon root endpoint ([#1122](https://github.com/stellar/js-stellar-sdk/pull/1122/)).

### Fixed
* When using a friendbot that points to a Horizon instance that has ledger metadata disabled, you can no longer extract the account sequence from the response. Instead, we hit RPC directly ([#1107](https://github.com/stellar/js-stellar-sdk/pull/1107/)).
* `rpc.Server.getEvents()` now correctly returns the `cursor` field at the top-level response ([#1124](https://github.com/stellar/js-stellar-sdk/pull/1124)).


## [v13.0.0](https://github.com/stellar/js-stellar-sdk/compare/v12.3.0...v13.0.0)
This is a direct re-tag of rc.2 with the only change being an upgrade to the `stellar-base` library to incorporate a patch release. Nonetheless, the entire changelog from the prior major version here is replicated for a comprehensive view on what's broken, added, and fixed.

### Breaking Changes
- We stopped supporting Node 18 explicitly a while ago, but now the Babelification of the codebase will transform to Node 18 instead of 16.

#### TypeScript Bindings: the `contract` module.
- `contract.AssembledTransaction#signAuthEntries` now takes an `address` instead of a `publicKey`. This brings the API more inline with its actual functionality: It can be used to sign all the auth entries for a particular _address_, whether that is the address of an account (public key) or a contract. ([#1044](https://github.com/stellar/js-stellar-sdk/pull/1044)).
- The `ClientOptions.signTransaction` type has been updated to reflect the latest [SEP-43](https://stellar.org/protocol/sep-43#wallet-interface-format) protocol, which matches the latest major version of Freighter and other wallets. It now accepts `address`, `submit`, and `submitUrl` options, and it returns a promise containing the `signedTxXdr` and the `signerAddress`. It now also returns an `Error` type if an error occurs during signing.
  * `basicNodeSigner` has been updated to reflect this new type.
- `ClientOptions.signAuthEntry` type has been updated to reflect the [SEP-43](https://stellar.org/protocol/sep-43#wallet-interface-format) protocol, which returns a promise containing the `signerAddress` in addition to the `signAuthEntry` that was returned previously. It also can return an `Error` type.
- `SentTransaction.init` and `new SentTransaction` now take _one_ (1) argument instead of _two_ (2). The first argument had previously been deprecated and ignored. To update:
```diff
-SentTransaction(nonsense, realStuff)
+SentTransaction(realStuff)
-new SentTransaction(nonsense, realStuff)
+new SentTransaction(realStuff)
```

#### Server APIs: the `rpc` and `Horizon` modules.
- Deprecated RPC APIs have been removed ([#1084](https://github.com/stellar/js-stellar-sdk/pull/1084)):
  * `simulateTransaction`'s `cost` field is removed
  * `rpc.Server.getEvents`'s `pagingToken` field is deprecated, use `cursor` instead
- Deprecated Horizon APIs have been removed (deprecated since [v10.0.1](https://github.com/stellar/js-stellar-sdk/releases/tag/v10.0.1), []()):
  * removed fields `transaction_count`, `base_fee`, and `base_reserve`
  * removed fields `num_accounts` and `amount` from assets
- The `SorobanRpc` import, previously deprecated, has been removed. You can import `rpc` instead:
```diff
-import { SorobanRpc } from '@stellar/stellar-sdk'
+import { rpc } from '@stellar/stellar-sdk'
// alternatively, you can also import from the `rpc` entrypoint:
import { Server } from '@stellar/stellar-sdk/rpc'
```

### Added

#### TypeScript Bindings: the `contract` module.
* `contract.Client` now has a static `deploy` method that can be used to deploy a contract instance from an existing uploaded/"installed" Wasm hash. The first arguments to this method are the arguments for the contract's `__constructor` method in accordance with [CAP-42](https://stellar.org/protocol/cap-42) ([#1086](https://github.com/stellar/js-stellar-sdk/pull/1086/)). For example, using the `increment` test contract as modified in https://github.com/stellar/soroban-test-examples/pull/2/files#diff-8734809100be3803c3ce38064730b4578074d7c2dc5fb7c05ca802b2248b18afR10-R45:
```typescript
const tx = await contract.Client.deploy(
  { counter: 42 },
  {
    networkPassphrase,
    rpcUrl,
    wasmHash: uploadedWasmHash,
    publicKey: someKeypair.publicKey(),
    ...basicNodeSigner(someKeypair, networkPassphrase),
  },
);
const { result: client } = await tx.signAndSend();
const t = await client.get();
expect(t.result, 42);
```
* `contract.AssembledTransaction#signAuthEntries` now allows you to override `authorizeEntry`. This can be used to streamline novel workflows using cross-contract auth. ([#1044](https://github.com/stellar/js-stellar-sdk/pull/1044)).

#### Server modules: the `rpc`, `Horizon`, and `stellartoml` modules.
* `Horizon.ServerApi` now has an `EffectType` exported so that you can compare and infer effect types directly ([#1099](https://github.com/stellar/js-stellar-sdk/pull/1099)).
* `Horizon.ServerApi.Trade` type now has a `type_i` field for type inference ([#1099](https://github.com/stellar/js-stellar-sdk/pull/1099)).
* All effects now expose their type as an exact string ([#947](https://github.com/stellar/js-stellar-sdk/pull/947)).
* `stellartoml.Resolver.resolve` now has a `allowedRedirects` option to configure the number of allowed redirects to follow when resolving a stellar toml file.
* `rpc.Server.getEvents` now returns a `cursor` field that matches `pagingToken` and `id`
* `rpc.Server.getTransactions` now returns a `txHash` field
* `rpc.Server` has two new methods:
  - `pollTransaction` to retry transaction retrieval ([#1092]https://github.com/stellar/js-stellar-sdk/pull/1092), and
  - `getSACBalance` to fetch the balance of a built-in Stellar Asset Contract token held by a contract ([#1046](https://github.com/stellar/js-stellar-sdk/pull/1046)), returning this schema:
```typescript
export interface BalanceResponse {
  latestLedger: number;
  /** present only on success, otherwise request malformed or no balance */
  balanceEntry?: {
    /** a 64-bit integer */
    amount: string;
    authorized: boolean;
    clawback: boolean;

    lastModifiedLedgerSeq?: number;
    liveUntilLedgerSeq?: number;
  };
}
```

#### New bundles without dependencies
- You can now build the browser bundle without various dependencies:
  * Set `USE_AXIOS=false` to build without the `axios` dependency: this will build `stellar-sdk-no-axios.js` and `stellar-sdk-no-axios.min.js` in the `dist/` directory, or just run `yarn build:browser:no-axios` to generate these files.
  * You can import Node packages without the `axios` dependency via `@stellar/stellar-sdk/no-axios`. For Node environments that don't support modern imports, use `@stellar/stellar-sdk/lib/no-axios/index`.
  * Set `USE_EVENTSOURCE=false` to build without the `eventsource` dependency: this will build `stellar-sdk-no-eventsource.js` and `stellar-sdk-no-eventsource.min.js` in the `dist/` directory, or just run `yarn build:browser:no-eventsource` to generate these files.
  * You can import Node packages without the `eventsource` dependency via `@stellar/stellar-sdk/no-eventsource`. For Node.js environments that don't support modern imports, use `@stellar/stellar-sdk/lib/no-eventsource/index`.
  * To use a minimal build without both Axios and EventSource, use `stellar-sdk-minimal.js` for the browser build and import from `@stellar/stellar-sdk/minimal` for the Node package.

### Fixed
- `contract.AssembledTransaction#nonInvokerSigningBy` now correctly returns contract addresses, in instances of cross-contract auth, rather than throwing an error. `sign` will ignore these contract addresses, since auth happens via cross-contract call ([#1044](https://github.com/stellar/js-stellar-sdk/pull/1044)).
- `buildInvocationTree` now correctly handles V2 contract creation and displays constructor args ([js-stellar-base#785](https://github.com/stellar/js-stellar-base/pull/785)).


## [v13.0.0-rc.2](https://github.com/stellar/js-stellar-sdk/compare/v13.0.0-rc.1...v13.0.0-rc.2)

### Breaking Changes
- The `ClientOptions.signTransaction` type has been updated to reflect the latest [SEP-43](https://stellar.org/protocol/sep-43#wallet-interface-format) protocol, which matches the latest major version of Freighter and other wallets. It now accepts `address`, `submit`, and `submitUrl` options, and it returns a promise containing the `signedTxXdr` and the `signerAddress`. It now also returns an `Error` type if an error occurs during signing.
  * `basicNodeSigner` has been updated to reflect the new type.
- `ClientOptions.signAuthEntry` type has also been updated to reflect the [SEP-43](https://stellar.org/protocol/sep-43#wallet-interface-format) protocol, which also returns a promise containing the `signerAddress` in addition to the `signAuthEntry` that was returned previously. It also can return an `Error` type.

### Added
* `contract.Client` now has a static `deploy` method that can be used to deploy a contract instance from an existing uploaded/"installed" Wasm hash. The first arguments to this method are the arguments for the contract's `__constructor` method in accordance with CAP-42 ([#1086](https://github.com/stellar/js-stellar-sdk/pull/1086/)).

For example, using the `increment` test contract as modified in https://github.com/stellar/soroban-test-examples/pull/2/files#diff-8734809100be3803c3ce38064730b4578074d7c2dc5fb7c05ca802b2248b18afR10-R45:
```typescript
  const tx = await contract.Client.deploy(
    { counter: 42 },
    {
      networkPassphrase,
      rpcUrl,
      wasmHash: uploadedWasmHash,
      publicKey: someKeypair.publicKey(),
      ...basicNodeSigner(someKeypair, networkPassphrase),
    },
  );
  const { result: client } = await tx.signAndSend();
  const t = await client.get();
  expect(t.result, 42);
```
* `Horizon.ServerApi` now has an `EffectType` exported so that you can compare and infer effect types directly ([#1099](https://github.com/stellar/js-stellar-sdk/pull/1099)).
* `Horizon.ServerApi.Trade` type now has a `type_i` field for type inference.
* All effects now expose their type as an exact string ([#947](https://github.com/stellar/js-stellar-sdk/pull/947)).
* `stellartoml-Resolver.resolve` now has a `allowedRedirects` option to configure the number of allowed redirects to follow when resolving a stellar toml file.


## [v13.0.0-rc.1](https://github.com/stellar/js-stellar-sdk/compare/v13.0.0-beta.1...v13.0.0-rc.1)

### Breaking Changes
- Deprecated RPC APIs have been removed ([#1084](https://github.com/stellar/js-stellar-sdk/pull/1084)):
  * `simulateTransaction`'s `cost` field is removed
  * `getEvents` returns a `cursor` field that matches `pagingToken` and `id`
  * `getTransactions` returns a `txHash` field
- Horizon Server API types: removed fields `transaction_count`, `base_fee`, and `base_reserve` (deprecated since [v10.0.1](https://github.com/stellar/js-stellar-sdk/releases/tag/v10.0.1))
- `SentTransaction.init` and `new SentTransaction` now take _one_ (1) argument instead of _two_ (2). The first argument had previously been deprecated and ignored. To update:
  ```diff
  -SentTransaction(nonsense, realStuff)
  +SentTransaction(realStuff)
  -new SentTransaction(nonsense, realStuff)
  +new SentTransaction(realStuff)
  ```
- `SorobanRpc` import, previously deprecated, has been removed. You can import `rpc` instead:
  ```diff
  -import { SorobanRpc } from '@stellar/stellar-sdk'
  +import { rpc } from '@stellar/stellar-sdk'
  ```

  As an alternative, you can also import from the `rpc` entrypoint:

  ```ts
  import { Server } from '@stellar/stellar-sdk/rpc'
  ```


### Added
- `rpc.Server` now has a `pollTransaction` method to retry transaction retrieval ([#1092]https://github.com/stellar/js-stellar-sdk/pull/1092).


## [v13.0.0-beta.1](https://github.com/stellar/js-stellar-sdk/compare/v12.3.0...v13.0.0-beta.1)

### Breaking Changes
- `contract.AssembledTransaction#signAuthEntries` now takes an `address` instead of a `publicKey`. This brings the API more inline with its actual functionality: It can be used to sign all the auth entries for a particular _address_, whether that is the address of an account (public key) or a contract. ([#1044](https://github.com/stellar/js-stellar-sdk/pull/1044)).
- The Node.js code will now Babelify to Node 18 instead of Node 16, but we stopped supporting Node 16 long ago so this shouldn't be a breaking change.

### Added
- You can now build the browser bundle without various dependencies:
  * Set `USE_AXIOS=false` to build without the `axios` dependency: this will build `stellar-sdk-no-axios.js` and `stellar-sdk-no-axios.min.js` in the `dist/` directory, or just run `yarn build:browser:no-axios` to generate these files.
  * You can import Node packages without the `axios` dependency via `@stellar/stellar-sdk/no-axios`. For Node environments that don't support modern imports, use `@stellar/stellar-sdk/lib/no-axios/index`.
  * Set `USE_EVENTSOURCE=false` to build without the `eventsource` dependency: this will build `stellar-sdk-no-eventsource.js` and `stellar-sdk-no-eventsource.min.js` in the `dist/` directory, or just run `yarn build:browser:no-eventsource` to generate these files.
  * You can import Node packages without the `eventsource` dependency via `@stellar/stellar-sdk/no-eventsource`. For Node.js environments that don't support modern imports, use `@stellar/stellar-sdk/lib/no-eventsource/index`.
  * To use a minimal build without both Axios and EventSource, use `stellar-sdk-minimal.js` for the browser build and import from `@stellar/stellar-sdk/minimal` for the Node package.
- `contract.AssembledTransaction#signAuthEntries` now allows you to override `authorizeEntry`. This can be used to streamline novel workflows using cross-contract auth. (#1044)
- `rpc.Server` now has a `getSACBalance` helper which lets you fetch the balance of a built-in Stellar Asset Contract token held by a contract ([#1046](https://github.com/stellar/js-stellar-sdk/pull/1046)):
```typescript
export interface BalanceResponse {
  latestLedger: number;
  /** present only on success, otherwise request malformed or no balance */
  balanceEntry?: {
    /** a 64-bit integer */
    amount: string;
    authorized: boolean;
    clawback: boolean;

    lastModifiedLedgerSeq?: number;
    liveUntilLedgerSeq?: number;
  };
}
```

### Fixed
- `contract.AssembledTransaction#nonInvokerSigningBy` now correctly returns contract addresses, in instances of cross-contract auth, rather than throwing an error. `sign` will ignore these contract addresses, since auth happens via cross-contract call ([#1044](https://github.com/stellar/js-stellar-sdk/pull/1044)).


## [v12.3.0](https://github.com/stellar/js-stellar-sdk/compare/v12.2.0...v12.3.0)

### Added
- `rpc.Server` now has a `getTransactions`, which has the same response schema as `getTransactions` except with bundles of transactions ([#1037](https://github.com/stellar/js-stellar-sdk/pull/1037)).
- `rpc.Server` now has a `getVersionInfo` method which reports version information of the RPC instance it is connected to ([#1028](https://github.com/stellar/js-stellar-sdk/issues/1028)):

```typescript
export interface GetVersionInfoResponse {
  version: string;
  commit_hash: string;
  build_time_stamp: string;
  captive_core_version: string;
  protocol_version: number;
}
```

### Fixed
- Lower authorization entry's default signature expiration to ~8min for security reasons ([#1023](https://github.com/stellar/js-stellar-sdk/pull/1023)).
- Remove `statusText` error check to broaden compatibility ([#1001](https://github.com/stellar/js-stellar-sdk/pull/1001)).
- Upgraded `stellar-base` which includes various fixes ([release notes](https://github.com/stellar/js-stellar-base/releases/tag/v12.1.1), [#1045](https://github.com/stellar/js-stellar-sdk/pull/1045)).


## [v12.2.0](https://github.com/stellar/js-stellar-sdk/compare/v12.1.0...v12.2.0)

### Fixed
- `@stellar/stellar-base` and its underlying dependency `@stellar/js-xdr` have been upgraded to their latest versions; reference their release notes ([v12.1.0](https://github.com/stellar/js-stellar-base/releases/tag/v12.1.0) and [v3.1.2](https://github.com/stellar/js-xdr/releases/tag/v3.1.2), respectively) for details ([#1013](https://github.com/stellar/js-stellar-sdk/pull/1013)).

### Added
- You can now pass custom headers to both `rpc.Server` and `Horizon.Server` ([#1013](https://github.com/stellar/js-stellar-sdk/pull/1013)):
```typescript
import { Server } from "@stellar/stellar-sdk/rpc";

const s = new Server("<some URL>", { headers: { "X-Custom-Header": "hello" }})
```
- `Horizon.Server` now supports the new `POST /transactions_async` endpoint via the `submitAsyncTransaction` method ([#989](https://github.com/stellar/js-stellar-sdk/pull/989)). Its purpose is to provide an immediate response to the submission rather than waiting for Horizon to determine its status. The response schema is as follows:
```typescript
interface SubmitAsyncTransactionResponse {
  // the submitted transaction hash
  hash: string;
  // one of "PENDING", "DUPLICATE", "TRY_AGAIN_LATER", or "ERROR"
  tx_status: string;
  // a base64-encoded xdr.TransactionResult iff `tx_status` is "ERROR"
  error_result_xdr: string;
}
```
- `rpc.Server` now has a `getFeeStats` method which retrieves fee statistics for a previous chunk of ledgers to provide users with a way to provide informed decisions about getting their transactions included in the following ledgers ([#998](https://github.com/stellar/js-stellar-sdk/issues/998)):
```typescript
export interface GetFeeStatsResponse {
  sorobanInclusionFee: FeeDistribution;
  inclusionFee: FeeDistribution;
  latestLedger: number; // uint32
}

interface FeeDistribution {
  max: string;  // uint64
  min: string;  // uint64
  mode: string; // uint64
  p10: string;  // uint64
  p20: string;  // uint64
  p30: string;  // uint64
  p40: string;  // uint64
  p50: string;  // uint64
  p60: string;  // uint64
  p70: string;  // uint64
  p80: string;  // uint64
  p90: string;  // uint64
  p95: string;  // uint64
  p99: string;  // uint64
  transactionCount: string; // uint32
  ledgerCount: number;      // uint32
}
```


## [v12.1.0](https://github.com/stellar/js-stellar-sdk/compare/v12.0.1...v12.1.0)

### Added
- `contract` now exports the `DEFAULT_TIMEOUT` ([#984](https://github.com/stellar/js-stellar-sdk/pull/984)).
- `contract.AssembledTransaction` now has:
  - `toXDR` and `fromXDR` methods for serializing the transaction to and from XDR. These methods should be used in place of `AssembledTransaction.toJSON` and `AssembledTransaction.fromJSON`for multi-auth signing. The JSON methods are now deprecated. **Note:** you must now call `simulate` on the transaction before the final `signAndSend` call after all required signatures are gathered when using the XDR methods ([#977](https://github.com/stellar/js-stellar-sdk/pull/977)).
  - a `restoreFootprint` method which accepts the `restorePreamble` returned when a simulation call fails due to some contract state that has expired. When invoking a contract function, one can now set `restore` to `true` in the `MethodOptions`. When enabled, a `restoreFootprint` transaction will be created and await signing when required ([#991](https://github.com/stellar/js-stellar-sdk/pull/991)).
  - separate `sign` and `send` methods so that you can sign a transaction without sending it (`signAndSend` still works as before; [#922](https://github.com/stellar/js-stellar-sdk/pull/992)).
- `contract.Client` now has a `txFromXDR` method which should be used in place of `txFromJSON` for multi-auth signing ([#977](https://github.com/stellar/js-stellar-sdk/pull/977)).

### Deprecated
- In `contract.AssembledTransaction`, `toJSON` and `fromJSON` should be replaced with `toXDR` and `fromXDR`.
- In `contract.Client`, `txFromJSON` should be replaced with `txFromXDR`.

### Fixed
- If you edit an `AssembledTransaction` with `tx.raw = cloneFrom(tx.build)`, the `tx.simulationData` will now be updated correctly ([#985](https://github.com/stellar/js-stellar-sdk/pull/985)).


## [v12.0.1](https://github.com/stellar/js-stellar-sdk/compare/v11.3.0...v12.0.1)

- This is a re-tag of `v12.0.0-rc.3` with dependency updates and a single new feature.

### Added
- `rpc.server.simulateTransaction` now supports an optional `stateChanges?: LedgerEntryChange[]` field ([#963](https://github.com/stellar/js-stellar-sdk/pull/963)):
  * If `Before` is omitted, it constitutes a creation, if `After` is omitted, it constitutes a deletions, note that `Before` and `After` cannot be omitted at the same time. Each item follows this schema:

```typescript
interface LedgerEntryChange {
  type: number;
  key: xdr.LedgerKey;
  before: xdr.LedgerEntry | null;
  after: xdr.LedgerEntry | null;
}
```


## [v12.0.0-rc.3](https://github.com/stellar/js-stellar-sdk/compare/v11.3.0...v12.0.0-rc.3)

### Breaking Changes

- `ContractClient` functionality previously added in [v11.3.0](https://github.com/stellar/js-stellar-sdk/releases/tag/v11.3.0) was exported in a non-standard way. You can now import it as any other `stellar-sdk` module ([#962](https://github.com/stellar/js-stellar-sdk/pull/962)):

```diff
-import { ContractClient } from '@stellar/stellar-sdk/lib/contract_client'
+import { contract } from '@stellar/stellar-sdk'
+const { Client } = contract
```

  Note that this top-level `contract` export is a container for ContractClient and related functionality. The `ContractClient` class is now available at `contract.Client`, as shown. Further note that there is a capitalized `Contract` export as well, which comes [from stellar-base](https://github.com/stellar/js-stellar-base/blob/b96281b9b3f94af23a913f93bdb62477f5434ccc/src/contract.js#L6-L19). You can remember which is which because capital-C `Contract` is a class, whereas lowercase-c `contract` is a container/module with a bunch of classes, functions, and types.

  Additionally, this is available from the `/contract` entrypoint, if your version of Node [and TypeScript](https://stackoverflow.com/a/70020984/249801) support [the `exports` declaration](https://nodejs.org/api/packages.html#exports). Finally, some of its exports have been renamed:

```diff
import {
-  ContractClient,
+  Client,
   AssembledTransaction,
-  ContractClientOptions,
+  ClientOptions,
   SentTransaction,
-} from '@stellar/stellar-sdk/lib/contract_client'
+} from '@stellar/stellar-sdk/contract'
```

- The `ContractSpec` class is now nested under the `contract` module, and has been **renamed** to `Spec` ([#962](https://github.com/stellar/js-stellar-sdk/pull/962)). Alternatively, you can import this from the `contract` entrypoint, if your version of Node [and TypeScript](https://stackoverflow.com/a/70020984/249801) support [the `exports` declaration](https://nodejs.org/api/packages.html#exports):

```diff
-import { ContractSpec } from '@stellar/stellar-sdk'
+import { contract } from '@stellar/stellar-sdk'
+const { Spec } = contract
// OR
+import { Spec } from '@stellar/stellar-sdk/contract'
```

- Previously, `AssembledTransaction.signAndSend()` would return a `SentTransaction` even if the transaction was never finalized. That is, if it successfully sent the transaction to the network, but the transaction was still `status: 'PENDING'`, then it would `console.error` an error message, but return the indeterminate transaction anyhow. **It now throws** a `SentTransaction.Errors.TransactionStillPending` error with that error message instead ([#962](https://github.com/stellar/js-stellar-sdk/pull/962)).

### Deprecated

- `SorobanRpc` module is now also exported as `rpc` ([#962](https://github.com/stellar/js-stellar-sdk/pull/962)). You can import it with either name for now, but `SorobanRpc` will be removed in a future release:

```diff
-import { SorobanRpc } from '@stellar/stellar-sdk'
+import { rpc } from '@stellar/stellar-sdk'
```

  You can also now import it at the `/rpc` entrypoint, if your version of Node [and TypeScript](https://stackoverflow.com/a/70020984/249801) support [the `exports` declaration](https://nodejs.org/api/packages.html#exports).

```diff
-import { SorobanRpc } from '@stellar/stellar-sdk'
-const { Api } = SorobanRpc
+import { Api } from '@stellar/stellar-sdk/rpc'
```

### Added
* New methods on `contract.Client` ([#960](https://github.com/stellar/js-stellar-sdk/pull/960)):
  - `from(opts: ContractClientOptions)` instantiates `contract.Client` by fetching the `contractId`'s WASM from the network to fill out the client's `ContractSpec`.
  - `fromWasm` and `fromWasmHash` methods to instantiate a `contract.Client` when you already have the WASM bytes or hash alongside the `contract.ClientOptions`.
* New methods on `rpc.Server` ([#960](https://github.com/stellar/js-stellar-sdk/pull/960)):
  - `getContractWasmByContractId` and `getContractWasmByHash` to retrieve a contract's WASM bytecode via its `contractId` or `wasmHash`, respectively.

### Fixed
* The breaking changes above (strictly speaking, they are not breaking changes because importing from the inner guts of the SDK is not supported) enable the `contract` module to be used in non-Node environments.


## [v12.0.0-rc.2](https://github.com/stellar/js-stellar-sdk/compare/v11.3.0...v12.0.0-rc.2)

**This update supports Protocol 21**. It is an additive change to the protocol so there are no true backwards incompatibilities, but your software may break if you encounter new unexpected fields from this Protocol ([#949](https://github.com/stellar/js-stellar-sdk/pull/949)).

### Breaking Changes
* The **default timeout for transaction calls is now set to 300 seconds (5 minutes)** from the previous default of 10 seconds. 10 seconds is often not enough time to review transactions before signing, especially in Freighter or using a hardware wallet like a Ledger, which would cause a `txTooLate` error response from the server. Five minutes is also the value used by the CLI, so this brings the two into alignment ([#956](https://github.com/stellar/js-stellar-sdk/pull/956)).

### Fixed
* Dependencies have been properly updated to pull in Protocol 21 XDR ([#959](https://github.com/stellar/js-stellar-sdk/pull/959)).


## [v12.0.0-rc.1](https://github.com/stellar/js-stellar-sdk/compare/v11.3.0...v12.0.0-rc.1)

### Breaking Changes
* **This update supports Protocol 21**. It is an additive change to the protocol so there are no true backwards incompatibilities, but your software may break if you encounter new unexpected fields from this Protocol ([#949](https://github.com/stellar/js-stellar-sdk/pull/949)).

### Fixed
* Each item in the `GetEventsResponse.events` list will now have a `txHash` item corresponding to the transaction hash that triggered a particular event ([#939](https://github.com/stellar/js-stellar-sdk/pull/939)).
* `ContractClient` now properly handles methods that take no arguments by making `MethodOptions` the only parameter, bringing it inline with the types generated by Soroban CLI's `soroban contract bindings typescript` ([#940](https://github.com/stellar/js-stellar-sdk/pull/940)).
* `ContractClient` now allows `publicKey` to be undefined ([#941](https://github.com/stellar/js-stellar-sdk/pull/941)).
* `SentTransaction` will only pass `allowHttp` if (and only if) its corresponding `AssembledTransaction#options` config allowed it ([#952](https://github.com/stellar/js-stellar-sdk/pull/952)).
* `SentTransaction` will now modify the time bounds of the transaction to be `timeoutInSeconds` seconds after the transaction has been simulated. Previously this was set when the transaction is built, before the simulation. This makes the time bounds line up with the timeout retry logic in `SentTransaction`.

## [v11.3.0](https://github.com/stellar/js-stellar-sdk/compare/v11.2.2...v11.3.0)

### Added
* Introduces an entire suite of helpers to assist with interacting with smart contracts ([#929](https://github.com/stellar/js-stellar-sdk/pull/929)):
  - `ContractClient`: generate a class from the contract specification where each Rust contract method gets a matching method in this class. Each method returns an `AssembledTransaction` that can be used to modify, simulate, decode results, and possibly sign, & submit the transaction.
  - `AssembledTransaction`: used to wrap a transaction-under-construction and provide high-level interfaces to the most common workflows, while still providing access to low-level transaction manipulation.
  - `SentTransaction`: transaction sent to the Soroban network, in two steps - initial submission and waiting for it to finalize to get the result (retried with exponential backoff)

### Fixed
* Upgrade underlying dependencies, including `@stellar/js-xdr` which should broaden compatibility to pre-ES2016 environments ([#932](https://github.com/stellar/js-stellar-sdk/pull/932), [#930](https://github.com/stellar/js-stellar-sdk/pull/930)).


### Fixed
* `SorobanRpc`: remove all instances of array-based parsing to conform to future breaking changes in Soroban RPC ([#924](https://github.com/stellar/js-stellar-sdk/pull/924)).


## [v11.2.2](https://github.com/stellar/js-stellar-sdk/compare/v11.2.1...v11.2.2)

### Fixed
* Event streaming tests now pass on Node 20, which seems to have tighter conformance to the spec ([#917](https://github.com/stellar/js-stellar-sdk/pull/917)).
* `@stellar/stellar-base` has been upgraded to its latest major version ([#918](https://github.com/stellar/js-stellar-sdk/pull/918), see [v11.0.0](https://github.com/stellar/js-stellar-base/releases/tag/v11.0.0) for release notes).


## [v11.2.1](https://github.com/stellar/js-stellar-sdk/compare/v11.2.0...v11.2.1)

### Fixed
* An unnecessary dependency has been removed which was causing a TypeScript error in certain environments ([#912](https://github.com/stellar/js-stellar-sdk/pull/912)).
* Dependencies have been upgraded (see [`stellar-base@v10.0.2`](https://github.com/stellar/js-stellar-base/releases/tag/v10.0.2) for release notes, [#913](https://github.com/stellar/js-stellar-sdk/pull/913)).


## [v11.2.0](https://github.com/stellar/js-stellar-sdk/compare/v11.1.0...v11.2.0)

### Added
* Support for the new, optional `diagnosticEventsXdr` field on the `SorobanRpc.Server.sendTransaction` method. The raw field will be present when using the `_sendTransaction` method, while the normal method will have an already-parsed `diagnosticEvents: xdr.DiagnosticEvent[]` field, instead ([#905](https://github.com/stellar/js-stellar-sdk/pull/905)).
* A new exported interface `SorobanRpc.Api.EventResponse` so that developers can type-check individual events ([#904](https://github.com/stellar/js-stellar-sdk/pull/904)).

### Updated
* Dependencies have been updated to their latest versions ([#906](https://github.com/stellar/js-stellar-sdk/pull/906), [#908](https://github.com/stellar/js-stellar-sdk/pull/908)).


## [v11.1.0](https://github.com/stellar/js-stellar-sdk/compare/v11.0.1...v11.1.0)

### Added
* `SorobanRpc.Server.simulateTransaction` now supports an optional `addlResources` parameter to allow users to specify additional resources that they want to include in a simulation ([#896](https://github.com/stellar/js-stellar-sdk/pull/896)).
* `ContractSpec` now has a `jsonSchema()` method to generate a [JSON Schema](https://json-schema.org/) for a particular contract specification ([#889](https://github.com/stellar/js-stellar-sdk/pull/889)).

### Fixed
* All dependencies have been updated to their latest versions, including `stellar-base` to [v10.0.1](https://github.com/stellar/js-stellar-base/releases/tag/v10.0.1) which included a small patch ([#897](https://github.com/stellar/js-stellar-sdk/pull/897)).


## [v11.0.1](https://github.com/stellar/js-stellar-sdk/compare/v10.2.1...v11.0.0)

### Fixed
* `SorobanRpc.Server.getEvents` uses the correct type for the start ledger.


## [v11.0.0](https://github.com/stellar/js-stellar-sdk/compare/v10.2.1...v11.0.0)

### Breaking Changes

* The package has been renamed to `@stellar/stellar-sdk`.
* The new minimum supported version is Node 18.
* The `PaymentCallBuilder` was incorrectly indicating that it would return a collection of `Payment` records, while [in reality](https://developers.stellar.org/api/horizon/resources/list-all-payments) it can return a handful of "payment-like" records ([#885](https://github.com/stellar/js-stellar-sdk/pull/885)).

### Fixed
* The `SorobanRpc.Server.getEvents` method now correctly parses responses without a `contractId` field set. The `events[i].contractId` field on an event is now optional, omitted if there was no ID for the event (e.g. system events;  ([#883](https://github.com/stellar/js-stellar-sdk/pull/883))).


## [v11.0.0-beta.6](https://github.com/stellar/js-stellar-sdk/compare/v11.0.0-beta.5...v11.0.0-beta.6)

### Fixed
* The `stellar-base` library has been upgraded to `beta.4` which contains a bugfix for large sequence numbers ([#877](https://github.com/stellar/js-stellar-sdk/pull/877)).
* The `SorobanRpc.Server.getTransaction()` method will now return the full response when encountering a `FAILED` transaction result ([#872](https://github.com/stellar/js-stellar-sdk/pull/872)).
* The `SorobanRpc.Server.getEvents()` method will correctly parse the event value (which is an `xdr.ScVal` rather than an `xdr.DiagnosticEvent`, see the modified `SorobanRpc.Api.EventResponse.value`; [#876](https://github.com/stellar/js-stellar-sdk/pull/876)).


## [v11.0.0-beta.5](https://github.com/stellar/js-stellar-sdk/compare/v11.0.0-beta.4...v11.0.0-beta.5)

### Breaking Changes
* The `soroban-client` library ([stellar/js-soroban-client](https://github.com/stellar/js-soroban-client)) has been merged into this package, causing significant breaking changes in the module structure ([#860](https://github.com/stellar/js-stellar-sdk/pull/860)):
  - The namespaces have changed to move each server-dependent component into its own module. Shared components (e.g. `TransactionBuilder`) are still in the top level, Horizon-specific interactions are in the `Horizon` namespace (i.e. `Server` is now `Horizon.Server`), and new Soroban RPC interactions are in the `SorobanRpc` namespace.
  - There is a [detailed migration guide](https://gist.github.com/Shaptic/5ce4f16d9cce7118f391fbde398c2f30) available to outline both the literal (i.e. necessary code changes) and philosophical (i.e. how to find certain functionality) changes needed to adapt to this merge.
* The `SorobanRpc.Server.prepareTransaction` and `SorobanRpc.assembleTransaction` methods no longer need an optional `networkPassphrase` parameter, because it is implicitly part of the transaction already ([#870](https://github.com/stellar/js-stellar-sdk/pull/870)).


## [v11.0.0-beta.4](https://github.com/stellar/js-stellar-sdk/compare/v11.0.0-beta.3...v11.0.0-beta.4)

### Fixed
- The `stellar-base` dependency has been pinned to a specific version to avoid incorrect semver resolution ([#867](https://github.com/stellar/js-stellar-sdk/pull/867)).


## [v11.0.0-beta.3](https://github.com/stellar/js-stellar-sdk/compare/v11.0.0-beta.2...v11.0.0-beta.3)

### Fixed
- Fix a webpack error preventing correct exports of the SDK for browsers ([#862](https://github.com/stellar/js-stellar-sdk/pull/862)).


## [v11.0.0-beta.2](https://github.com/stellar/js-stellar-sdk/compare/v11.0.0-beta.1...v11.0.0-beta.2)

### Breaking Changes
- Certain effects have been renamed to align better with the "tense" that other structures have ([#844](https://github.com/stellar/js-stellar-sdk/pull/844)):
  * `DepositLiquidityEffect` -> `LiquidityPoolDeposited`
  * `WithdrawLiquidityEffect` -> `LiquidityPoolWithdrew`
  * `LiquidityPoolTradeEffect` -> `LiquidityPoolTrade`
  * `LiquidityPoolCreatedEffect` -> `LiquidityPoolCreated`
  * `LiquidityPoolRevokedEffect` -> `LiquidityPoolRevoked`
  * `LiquidityPoolRemovedEffect` -> `LiquidityPoolRemoved`

### Add
- New effects have been added to support Protocol 20 (Soroban) ([#842](https://github.com/stellar/js-stellar-sdk/pull/842)):
  * `ContractCredited` occurs when a Stellar asset moves **into** its corresponding Stellar Asset Contract instance
  * `ContractDebited` occurs when a Stellar asset moves **out of** its corresponding Stellar Asset Contract instance
- Asset stat records (`ServerApi.AssetRecord`) contain two new fields to support the Protocol 20 (Soroban) release ([#841](https://github.com/stellar/js-stellar-sdk/pull/841)):
  * `num_contracts` - the integer quantity of contracts that hold this asset
  * `contracts_amount` - the total units of that asset held by contracts
- New operation responses ([#845](https://github.com/stellar/js-stellar-sdk/pull/845)):
  * `invokeHostFunction`: see `Horizon.InvokeHostFunctionOperationResponse`
  * `bumpFootprintExpiration`: see `Horizon.BumpFootprintExpirationOperationResponse`
  * `restoreFootprint`: see `Horizon.RestoreFootprintOperationResponse`
  * You can refer to the actual definitions for details, but the gist of the schemas is below:
```ts
interface InvokeHostFunctionOperationResponse {
  function: string;
  parameters: {
    value: string;
    type: string;
  }[];
  address: string;
  salt: string;
  asset_balance_changes: {
    type: string;
    from: string;
    to: string;
    amount: string;
  }[];
}
interface BumpFootprintExpirationOperationResponse {
  ledgersToExpire: string;
}
interface RestoreFootprintOperationResponse {};
```

### Fixed
- Some effect definitions that were missing have been added ([#842](https://github.com/stellar/js-stellar-sdk/pull/842)):
  * `ClaimableBalanceClawedBack` is now defined
  * `type EffectRecord` now has all of the effect types
- The `stellar-base` library has been upgraded to support the latest Protocol 20 XDR schema and all Soroban functionality ([]()).


## [v11.0.0-beta.1](https://github.com/stellar/js-stellar-sdk/compare/v11.0.0-beta.0...v11.0.0-beta.1)

### Update

- Bundle size has decreased by dropping unnecessary dependencies (`lodash`: [#822](https://github.com/stellar/js-stellar-sdk/pull/822), `es6-promise`: [#823](https://github.com/stellar/js-stellar-sdk/pull/823), polyfills: [#825](https://github.com/stellar/js-stellar-sdk/pull/825), `detect-node`: [#831](https://github.com/stellar/js-stellar-sdk/issues/831)).
- Dependencies (including `stellar-base`) have been updated to their latest versions ([#825](https://github.com/stellar/js-stellar-sdk/pull/825), [#827](https://github.com/stellar/js-stellar-sdk/pull/827)).


## [v11.0.0-beta.0](https://github.com/stellar/js-stellar-sdk/compare/v10.4.1...v11.0.0-beta.0)

This version is marked by a major version bump because of the significant upgrades to underlying dependencies. While there should be no noticeable API changes from a downstream perspective, there may be breaking changes in the way that this library is bundled.

### Update

- Build system has been overhauled to support Webpack 5 ([#814](https://github.com/stellar/js-stellar-sdk/pull/814)).
- `stellar-base` has been updated to its corresponding overhaul ([#818](https://github.com/stellar/js-stellar-sdk/pull/818)).

### Fix

- Missing fields have been added to certain API responses ([#801](https://github.com/stellar/js-stellar-sdk/pull/801) and [#797](https://github.com/stellar/js-stellar-sdk/pull/797)).


## [v10.4.1](https://github.com/stellar/js-stellar-sdk/compare/v10.4.0...v10.4.1)

### Update

- Bumps `stellar-base` version to [v8.2.2](https://github.com/stellar/js-stellar-base/releases/tag/v8.2.2) to include latest fix: enabling fast signing in service workers ([#806](https://github.com/stellar/js-stellar-sdk/pull/806)).


## [v10.4.0](https://github.com/stellar/js-stellar-sdk/compare/v10.3.0...v10.4.0)

### Add

- Add [SEP-1](https://stellar.org/protocol/sep-1) fields to `StellarTomlResolver` for type checks ([#794](https://github.com/stellar/js-stellar-sdk/pull/794)).
- Add support for passing `X-Auth-Token` as a custom header ([#795](https://github.com/stellar/js-stellar-sdk/pull/795)).

### Update

- Bumps `stellar-base` version to [v8.2.1](https://github.com/stellar/js-stellar-base/releases/tag/v8.2.1) to include latest fixes.


## [v10.3.0](https://github.com/stellar/js-stellar-sdk/compare/v10.2.0...v10.3.0)

### Fix

- Adds `successful` field to transaction submission response ([#790](https://github.com/stellar/js-stellar-sdk/pull/790)).

### Update

- Bumps `stellar-base` version to [v8.2.0](https://github.com/stellar/js-stellar-base/releases/tag/v8.2.0) to include CAP-40 support in `Operation.setOptions`.


## [v10.2.0](https://github.com/stellar/js-stellar-sdk/compare/v10.1.2...v10.2.0)

### Fix

- Adds the missing `successful` field to transaction responses ([#790](https://github.com/stellar/js-stellar-sdk/pull/790)).

### Update

- Bumps `stellar-base` version to [v8.1.0](https://github.com/stellar/js-stellar-base/releases/tag/v8.1.0) to include bug fixes and latest XDR changes.


## [v10.1.2](https://github.com/stellar/js-stellar-sdk/compare/v10.1.1...v10.1.2)

### Fix

- Upgrades the `eventsource` dependency to fix a critical security vulnerability ([#783](https://github.com/stellar/js-stellar-sdk/pull/783)).


## [v10.1.1](https://github.com/stellar/js-stellar-sdk/compare/v10.1.0...v10.1.1)

### Fix

- Reverts a change from [v10.1.0](#v10.1.0) which caused streams to die prematurely ([#780](https://github.com/stellar/js-stellar-sdk/pull/780)).
- Bumps `stellar-base` version to [v8.0.1](https://github.com/stellar/js-stellar-base/releases/tag/v8.0.1) to include latest bugfixes.


## [v10.1.0](https://github.com/stellar/js-stellar-sdk/compare/v10.0.1...v10.1.0-beta.0)

This is a promotion from the beta version without changes, besides upgrading the underlying [stellar-base@v8.0.0](https://github.com/stellar/js-stellar-base/releases/tag/v8.0.0) to its stable release.


## [v10.1.0-beta.0](https://github.com/stellar/js-stellar-sdk/compare/v10.0.1...v10.1.0-beta.0)

### Add

- Add a way to filter offers by seller: `OfferCallBuilder.seller(string)`, corresponding to `GET /offers?seller=<string>` ([#773](https://github.com/stellar/js-stellar-sdk/pull/773)).

### Add

- Support for Protocol 19 ([#775](https://github.com/stellar/js-stellar-sdk/pull/775)):
  * new precondition fields on a `TransactionResponse`
  * new account fields on `AccountResponse` and `AccountRecord`
  * bumping `stellar-base` to the latest beta version

### Fix

- Add missing field to account responses: `last_modified_time` which is the time equivalent of the existing `last_modified_ledger` ([#770](https://github.com/stellar/js-stellar-sdk/pull/770)).
- Stop opening extra connections when SSE streams receive `event: close` events ([#772](https://github.com/stellar/js-stellar-sdk/pull/772)).
- Fix SSE streams not loading under React Native (thank you, @hunterpetersen!) ([#761](https://github.com/stellar/js-stellar-sdk/pull/761)).


## [v10.0.1](https://github.com/stellar/js-stellar-sdk/compare/v10.0.0...v10.0.1)

### Fix

- Add missing fields to the `LedgerRecord`: `successful_transaction_count` and `failed_transaction_count` ([#740](https://github.com/stellar/js-stellar-sdk/pull/740)). Note that this also marks several fields as _deprecated_ because they don't actually exist in the Horizon API response:
  * `transaction_count`: superceded by the sum of the aforementioned fields
  * `base_fee`: superceded by the `base_fee_in_stroops` field
  * `base_reserve`: superceded by the `base_reserve_in_stroops` field

These deprecated fields will be removed in the next major version. It's unlikely that this breaking change should affect anyone, as these fields have likely been missing/invalid for some time.

### Update
- Update a number of dependencies that needed various security updates:
  * several dependencies bumped their patch version ([#736](https://github.com/stellar/js-stellar-sdk/pull/736), [#684](https://github.com/stellar/js-stellar-sdk/pull/684), [#672](https://github.com/stellar/js-stellar-sdk/pull/672), [#666](https://github.com/stellar/js-stellar-sdk/pull/666), [#644](https://github.com/stellar/js-stellar-sdk/pull/644), [#622](https://github.com/stellar/js-stellar-sdk/pull/622))
  * axios has been bumped to 0.25.0 without causing breaking changes ([#742](https://github.com/stellar/js-stellar-sdk/pull/742))
  * the `karma` suite of packages has been updated to the latest major version ([#743](https://github.com/stellar/js-stellar-sdk/pull/743))

All of the dependencies in question besides `axios` were _developer_ dependencies, so there never was downstream security impact nor will there be downstream upgrade impact.


## [v10.0.0](https://github.com/stellar/js-stellar-sdk/compare/v9.1.0...v10.0.0)

This release introduces breaking changes from `stellar-base`. It adds **unconditional support for muxed accounts**. Please refer to the corresponding [release notes](https://github.com/stellar/js-stellar-base/releases/tag/v7.0.0) for details on the breaking changes there.

### Breaking Updates

- Upgrades the stellar-base library to v7.0.0 ([#735](https://github.com/stellar/js-stellar-sdk/pull/735)).

- Removes the `AccountResponse.createSubaccount` method since this is also gone from the underlying `Account` interface. The `stellar-base` release notes describe alternative construction methods ([#735](https://github.com/stellar/js-stellar-sdk/pull/735)).

### Fix

- Use the right string for liquidity pool trades ([#734](https://github.com/stellar/js-stellar-sdk/pull/734)).


## [v9.1.0](https://github.com/stellar/js-stellar-sdk/compare/v9.0.1...v9.1.0)

### Add

- Adds a way to filter liquidity pools by participating account: `server.liquidityPools.forAccount(id)` ([#727](https://github.com/stellar/js-stellar-sdk/pull/727)).

### Updates

- Updates the following SEP-10 utility functions to include [client domain verification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md#verifying-the-client-domain) functionality ([#720](https://github.com/stellar/js-stellar-sdk/pull/720)):
  - `Utils.buildChallengeTx()` accepts the `clientDomain` and `clientSigningKey` optional parameters
  - `Utils.readChallengeTx()` parses challenge transactions containing a `client_domain` ManageData operation
  - `Utils.verifyChallengeTxSigners()` verifies an additional signature from the `clientSigningKey` keypair if a `client_domain` Manage Data operation is included in the challenge

- Bumps `stellar-base` version to [v6.0.6](https://github.com/stellar/js-stellar-base/releases/tag/v6.0.6).

### Fix

- Fixes the `type_i` enumeration field to accurately reflect liquidity pool effects ([#723](https://github.com/stellar/js-stellar-sdk/pull/723)).

- Upgrades axios dependency to v0.21.4 to alleviate security concern ([GHSA-cph5-m8f7-6c5x](https://github.com/advisories/GHSA-cph5-m8f7-6c5x), [#724](https://github.com/stellar/js-stellar-sdk/pull/724)).

- Publish Bower package to [stellar/bower-js-stellar-sdk](https://github.com/stellar/bower-js-stellar-sdk) ([#725](https://github.com/stellar/js-stellar-sdk/pull/725)).


## [v9.0.1](https://github.com/stellar/js-stellar-sdk/compare/v9.0.0-beta.1...v9.0.1)

This stable release adds **support for Protocol 18**. For details, you can refer to [CAP-38](https://stellar.org/protocol/cap-38) for XDR changes and [this document](https://docs.google.com/document/d/1pXL8kr1a2vfYSap9T67R-g72B_WWbaE1YsLMa04OgoU/view) for changes to the Horizon API.

Refer to the release notes for the betas (e.g. [v9.0.0-beta.0](https://github.com/stellar/js-stellar-sdk/releases/v9.0.0-beta.0)) for a comprehensive list of changes to this library.

### Fix

- Corrects the `reserves` field on `LiquidityPoolRecord`s to be an array ([#715](https://github.com/stellar/js-stellar-sdk/pull/715)).
- Bumps the `stellar-base` dependency to [v6.0.4](https://github.com/stellar/js-stellar-base/releases/tag/v6.0.4) ([#715](https://github.com/stellar/js-stellar-sdk/pull/715)).


## [v9.0.0-beta.1](https://github.com/stellar/js-stellar-sdk/compare/v9.0.0-beta.0...v9.0.0-beta.1)

### Add

- Add `/liquidity_pools/:id/trades` endpoint ([#710](https://github.com/stellar/js-stellar-sdk/pull/710))

### Updates

- Updates the following SEP-10 utility functions to be compliant with the protocols ([#709](https://github.com/stellar/js-stellar-sdk/pull/709/), [stellar-protocol/#1036](https://github.com/stellar/stellar-protocol/pull/1036))
    - Updated `utils.buildChallengeTx()` to accept muxed accounts (`M...`) for client account IDs
    - Updated `utils.buildChallengeTx()` to accept a `memo` parameter to attach to the challenge transaction
    - Updated `utils.readChallengeTx()` to provide a `memo` property in the returned object
    - Updated `utils.readChallengeTx()` to validate challenge transactions with muxed accounts (`M...`) as the client account ID

### Fix

- Drops the `chai-http` dependency to be only for developers ([#707](https://github.com/stellar/js-stellar-sdk/pull/707)).

## [v9.0.0-beta.0](https://github.com/stellar/js-stellar-sdk/compare/v8.2.5...v9.0.0-beta.0)

This beta release adds **support for Automated Market Makers**. For details, you can refer to [CAP-38](https://stellar.org/protocol/cap-38) for XDR changes and [this document](https://docs.google.com/document/d/1pXL8kr1a2vfYSap9T67R-g72B_WWbaE1YsLMa04OgoU/view) for detailed changes to the Horizon API.

### Add

- Introduced a `LiquidityPoolCallBuilder` to make calls to a new endpoint:
  * `/liquidity_pools[?reserves=...]` - a collection of liquidity pools, optionally filtered by one or more assets ([#682](https://github.com/stellar/js-stellar-sdk/pull/682))
  * `/liquidity_pools/:id` - a specific liquidity pool ([#687](https://github.com/stellar/js-stellar-sdk/pull/687))

- Expanded the `TransactionCallBuilder`, `OperationCallBuilder`, and `EffectsCallBuilder`s to apply to specific liquidity pools ([#689](https://github.com/stellar/js-stellar-sdk/pull/689)). This corresponds to the following new endpoints:
  * `/liquidity_pools/:id/transactions`
  * `/liquidity_pools/:id/operations`
  * `/liquidity_pools/:id/effects`

- Expanded the `TradesCallBuilder` to support fetching liquidity pool trades and accepts a new `trade_type` filter ([#685](https://github.com/stellar/js-stellar-sdk/pull/685)):
  * `/trades?trade_type={orderbook,liquidity_pools,all}`. By default, the filter is `all`, including both liquidity pool and orderbook records.
  * A liquidity pool trade contains the following fields:
    - `liquidity_pool_fee_bp`: LP fee expressed in basis points, and *either*
    - `base_liquidity_pool_id` or `counter_liquidity_pool_id`

- Added new effects related to liquidity pools ([#690](https://github.com/stellar/js-stellar-sdk/pull/690)):
  * `DepositLiquidityEffect`
  * `WithdrawLiquidityEffect`
  * `LiquidityPoolTradeEffect`
  * `LiquidityPoolCreatedEffect`
  * `LiquidityPoolRemovedEffect`
  * `LiquidityPoolRevokedEffect`

- Added new responses related to liquidity pool operations ([#692](https://github.com/stellar/js-stellar-sdk/pull/692)):
  * `DepositLiquidityOperationResponse`
  * `WithdrawLiquidityOperationResponse`

### Updates

- Updated the underlying `stellar-base` library to [v6.0.1](https://github.com/stellar/js-stellar-base/releases/tag/v6.0.1) to include CAP-38 changes ([#681](https://github.com/stellar/js-stellar-sdk/pull/681)).

- Updated various developer dependencies to secure versions ([#671](https://github.com/stellar/js-stellar-sdk/pull/671)).

- Updated `AccountResponse` to include liquidity pool shares in its `balances` field ([#688](https://github.com/stellar/js-stellar-sdk/pull/688)).

- Updated `AccountCallBuilder` to allow filtering based on participation in a certain liquidity pool ([#688](https://github.com/stellar/js-stellar-sdk/pull/688)), corresponding to the following new filter:
  * `/accounts?reserves=[...list of assets...]`

- Updated `RevokeSponsorshipOperationResponse` to contain an optional attribute `trustline_liquidity_pool_id`, for when a liquidity pool trustline is revoked ([#690](https://github.com/stellar/js-stellar-sdk/pull/690)).

### Breaking changes

- A `TradeRecord` can now correspond to two different types of trades and has changed ([#685](https://github.com/stellar/js-stellar-sdk/pull/685)):
  * `Orderbook` (the existing structure)
    - `counter_offer_id` and `base_offer_id` only show up in these records
    - the redundant `offer_id` field was removed; it matches `base_offer_id`
  * `LiquidityPool` (new)
    - `base_account` xor `counter_account` will appear in these records
  * `price` fields changed from `number`s to `string`s
  * The links to `base` and `counter` can now point to *either* an account or a liquidity pool

- An account's `balances` array can now include a new type ([#688](https://github.com/stellar/js-stellar-sdk/pull/688)):
  * `asset_type` can now be `liquidity_pool_shares`
  * The following fields are *not* included in pool share balances:
    - `buying_liabilities`
    - `selling_liabilities`
    - `asset_code`
    - `asset_issue`

- The `ChangeTrustOperationResponse` has changed ([#688](https://github.com/stellar/js-stellar-sdk/pull/688), [#692](https://github.com/stellar/js-stellar-sdk/pull/692)):
  * `asset_type` can now be `liquidity_pool_shares`
  * `asset_code`, `asset_issuer`, and `trustee` are now optional
  * `liquidity_pool_id` is a new optional field

- The trustline effects (`TrustlineCreated`, `TrustlineUpdated`, `TrustlineRevoked`) have changed ([#690](https://github.com/stellar/js-stellar-sdk/pull/690)):
  * the asset type can now be `liquidity_pool_shares`
  * they can optionally include a `liquidity_pool_id`

- Trustline sponsorship effects (`TrustlineSponsorshipCreated`, `TrustlineSponsorshipUpdated`, `TrustlineSponsorshipRemoved`) have been updated ([#690](https://github.com/stellar/js-stellar-sdk/pull/690)):
  * the `asset` field is now optional, and is replaced by
  * the `liquidity_pool_id` field for liquidity pools


## [v8.2.5](https://github.com/stellar/js-stellar-sdk/compare/v8.2.4...v8.2.5)

### Update
- The `js-stellar-base` library has been updated to [v5.3.2](https://github.com/stellar/js-stellar-base/releases/tag/v5.3.2), which fixes a muxed account bug and updates vulnerable dependencies ([#670](https://github.com/stellar/js-stellar-sdk/pull/670)).


## [v8.2.4](https://github.com/stellar/js-stellar-sdk/compare/v8.2.3...v8.2.4)

### Fix
- Utils.readTransactionTx now checks timebounds with a 5-minute grace period to account for clock drift.


## [v8.2.3](https://github.com/stellar/js-stellar-sdk/compare/v8.2.2...v8.2.3)

### Fix
- Fix server signature verification in `Utils.readChallengeTx`. The function was
not verifying the server account had signed the challenge transaction.


## [v8.2.2](https://github.com/stellar/js-stellar-sdk/compare/v8.2.1...v8.2.2)

### Fix
- Fixes a breaking bug introduced in v8.2.0 in which `AccountResponse` no longer conformed to the `StellarBase.Account` interface, which was updated in [stellar-base@v5.2.0](https://github.com/stellar/js-stellar-base/releases/tag/v5.2.0) [(#655)](https://github.com/stellar/js-stellar-sdk/pull/655).


## [v8.2.1](https://github.com/stellar/js-stellar-sdk/compare/v8.2.0...v8.2.1)

### Fix
- A defunct query paramater (`?c=[...]`) has been removed now that Horizon properly sends Cache-Control headers [(#652)](https://github.com/stellar/js-stellar-sdk/pull/652).


## [v8.2.0](https://github.com/stellar/js-stellar-sdk/compare/v8.1.1...v8.2.0)

### Add
- Added support for querying the relevant transactions and operations for a claimable balance [(#628)](https://github.com/stellar/js-stellar-sdk/pull/628):
  * `TransactionCallBuilder.forClaimableBalance()`: builds a query to `/claimable_balances/:id/transactions/`
  * `OperationCallBuilder.forClaimableBalance()`: builds a query to `/claimable_balances/:id/operations/`

- Added support for new stat fields on the `/assets` endpoint [(#628)](https://github.com/stellar/js-stellar-sdk/pull/628):
  * `accounts` - a breakdown of accounts using this asset by authorization type
  * `balances` - a breakdown of balances by account authorization type
  * `num_claimable_balances` - the number of pending claimable balances
  * `claimable_balances_amount` - the total balance of pending claimable balances

- Added types for all Effects supported as an enum, and moved `Trade`, `Asset`, `Offer`, and `Account` types to separate files [(#635)](https://github.com/stellar/js-stellar-sdk/pull/635).

### Update
- Upgraded `js-stellar-base` package to version `^5.2.1` from `^5.1.0`, refer to its [release notes](https://github.com/stellar/js-stellar-base/releases/tag/v5.2.0) for more [(#639)](https://github.com/stellar/js-stellar-sdk/pull/639):
  * opt-in **support for muxed accounts** ([SEP-23](https://stellar.org/protocol/sep-23))
  * exposing the `AuthClawbackEnabled` flag to Typescript to **complete Protocol 17 support**
  * fixing a public key parsing regression

- Exposed more Protocol 17 (CAP-35) operations [(#633)](https://github.com/stellar/js-stellar-sdk/pull/633):
  * The `/accounts` endpoint now resolves the `flags.auth_clawback_enabled` field.
  * The operation responses for `clawback`, `clawbackClaimableBalance`, and `setTrustLineFlags` are now defined.
  * The operation response for `setOptions` has been updated to show `auth_clawback_enabled`.

## [v8.1.1](https://github.com/stellar/js-stellar-sdk/compare/v8.1.0...v8.1.1)

### Fix

- Upgraded `js-stellar-base` package to version `^5.1.0` from `^5.0.0` to expose the Typescript hints for CAP-35 operations [(#629)](https://github.com/stellar/js-stellar-sdk/pull/629).


## [v8.1.0](https://github.com/stellar/js-stellar-sdk/compare/v8.0.0...v8.1.0)

### Update

- Upgraded `js-stellar-base` package to version `^5.0.0` from `^4.0.3` to support new CAP-35 operations [(#624)](https://github.com/stellar/js-stellar-sdk/pull/624)


## [v8.0.0](https://github.com/stellar/js-stellar-sdk/compare/v7.0.0...v8.0.0)

### Breaking

- Updates the SEP-10 utility function parameters to support [SEP-10 v3.1](https://github.com/stellar/stellar-protocol/commit/6c8c9cf6685c85509835188a136ffb8cd6b9c11c) [(#607)](https://github.com/stellar/js-stellar-sdk/pull/607)
  - A new required `webAuthDomain` parameter was added to the following functions
    - `utils.buildChallengeTx()`
    - `utils.readChallengeTx()`
    - `utils.verifyChallengeTxThreshold()`
    - `utils.verifyChallengeTxSigners()`
  - The `webAuthDomain` parameter is expected to match the value of the Manage Data operation with the 'web_auth_domain' key, if present

### Fix

- Fixes bug where the first Manage Data operation in a challenge transaction could have a null value [(#591)](https://github.com/stellar/js-stellar-sdk/pull/591)

### Update

- Upgraded `axios` package to version `^0.21.1` from `^0.19.0` to fix security vulnerabilities [(#608)](https://github.com/stellar/js-stellar-sdk/pull/608)

- Upgraded `js-stellar-base` package to version `^4.0.3` from `^4.0.0` to allow accounts with a balance of zero [(#616)](https://github.com/stellar/js-stellar-sdk/pull/616)

## [v7.0.0](https://github.com/stellar/js-stellar-sdk/compare/v6.2.0...v7.0.0)

This release includes a major-version increase due to breaking changes included.

### Breaking

- Updates the SEP-10 utility function parameters and return values to support [SEP-10 v3.0](https://github.com/stellar/stellar-protocol/commit/9d121f98fd2201a5edfe0ed2befe92f4bf88bfe4)
  - The following functions replaced the `homeDomain` parameter with `homeDomains` (note: plural):
    - `utils.readChallengeTx()`
    - `utils.verifyChallengeTxThreshold()`
    - `utils.verifyChallengeTxSigners()`
  - `utils.readChallengeTx()` now returns an additional object attribute, `matchedHomeDomain`

### Update

- Update challenge transaction helpers for SEP0010 v3.0.0. ([#596](https://github.com/stellar/js-stellar-sdk/pull/596))
   * Restore `homeDomain` validation in `readChallengeTx()`.

## [v6.2.0](https://github.com/stellar/js-stellar-sdk/compare/v6.1.0...v6.2.0)

### Update

- Update challenge transaction helpers for SEP0010 v2.1.0. ([#581](https://github.com/stellar/js-stellar-sdk/issues/581))
   * Remove verification of home domain.
   * Allow additional manage data operations that have the source account set as the server key.

## [v6.1.0](https://github.com/stellar/js-stellar-sdk/compare/v6.0.0...v6.1.0)

### Update

- Update claim predicate fields to match Horizon 1.9.1 ([#575](https://github.com/stellar/js-stellar-sdk/pull/575)).

## [v6.0.0](https://github.com/stellar/js-stellar-sdk/compare/v5.0.4...v6.0.0)

### Add

- Add support for claimable balances ([#572](https://github.com/stellar/js-stellar-sdk/pull/572)).
Extend server class to allow loading claimable balances from Horizon. The following functions are available:

```
server.claimableBalances();
server.claimableBalances().claimant(claimant);
server.claimableBalances().sponsor(sponsorID);
server.claimableBalances().asset(asset);
server.claimableBalances().claimableBalance(balanceID);
```
-  Add the following attributes to `AccountResponse` ([#572](https://github.com/stellar/js-stellar-sdk/pull/572)):
    * `sponsor?: string`
    * `num_sponsoring: number`
    * `num_sponsored: number`

- Add the optional attribute `sponsor` to `AccountSigner`, `BalanceLineAsset`, `ClaimableBalanceRecord`, and `OfferRecord` ([#572](https://github.com/stellar/js-stellar-sdk/pull/572)).

- Add `sponsor` filtering support for `offers` and `accounts` ([#572](https://github.com/stellar/js-stellar-sdk/pull/572)).
    * `server.offers().sponsor(accountID)`
    * `server.accounts().sponsor(accountID)`

- Extend operation responses to support new operations ([#572](https://github.com/stellar/js-stellar-sdk/pull/572)).
    * `create_claimable_balance` with the following fields:
        * `asset` - asset available to be claimed (in canonical form),
        * `amount` - amount available to be claimed,
        * `claimants` - list of claimants with predicates (see below):
            * `destination` - destination account ID,
            * `predicate` - predicate required to claim a balance (see below).
    * `claim_claimable_balance` with the following fields:
        * `balance_id` - unique ID of balance to be claimed,
        * `claimant` - account ID of a claimant.
    * `begin_sponsoring_future_reserves` with the following fields:
        * `sponsored_id` - account ID for which future reserves will be sponsored.
    * `end_sponsoring_future_reserves` with the following fields:
        * `begin_sponsor` - account sponsoring reserves.
    * `revoke_sponsorship` with the following fields:
        * `account_id` - if account sponsorship was revoked,
        * `claimable_balance_id` - if claimable balance sponsorship was revoked,
        * `data_account_id` - if account data sponsorship was revoked,
        * `data_name` - if account data sponsorship was revoked,
        * `offer_id` - if offer sponsorship was revoked,
        * `trustline_account_id` - if trustline sponsorship was revoked,
        * `trustline_asset` - if trustline sponsorship was revoked,
        * `signer_account_id` - if signer sponsorship was revoked,
        * `signer_key` - if signer sponsorship was revoked.

- Extend effect responses to support new effects ([#572](https://github.com/stellar/js-stellar-sdk/pull/572)).
    * `claimable_balance_created` with the following fields:
        * `balance_id` - unique ID of claimable balance,
        * `asset` - asset available to be claimed (in canonical form),
        * `amount` - amount available to be claimed.
    * `claimable_balance_claimant_created` with the following fields:
        * `balance_id` - unique ID of a claimable balance,
        * `asset` - asset available to be claimed (in canonical form),
        * `amount` - amount available to be claimed,
        * `predicate` - predicate required to claim a balance (see below).
    * `claimable_balance_claimed` with the following fields:
        * `balance_id` - unique ID of a claimable balance,
        * `asset` - asset available to be claimed (in canonical form),
        * `amount` - amount available to be claimed,
    * `account_sponsorship_created` with the following fields:
        * `sponsor` - sponsor of an account.
    * `account_sponsorship_updated` with the following fields:
        * `new_sponsor` - new sponsor of an account,
        * `former_sponsor` - former sponsor of an account.
    * `account_sponsorship_removed` with the following fields:
        * `former_sponsor` - former sponsor of an account.
    * `trustline_sponsorship_created` with the following fields:
        * `sponsor` - sponsor of a trustline.
    * `trustline_sponsorship_updated` with the following fields:
        * `new_sponsor` - new sponsor of a trustline,
        * `former_sponsor` - former sponsor of a trustline.
    * `trustline_sponsorship_removed` with the following fields:
        * `former_sponsor` - former sponsor of a trustline.
    * `claimable_balance_sponsorship_created` with the following fields:
        * `sponsor` - sponsor of a claimable balance.
    * `claimable_balance_sponsorship_updated` with the following fields:
        * `new_sponsor` - new sponsor of a claimable balance,
        * `former_sponsor` - former sponsor of a claimable balance.
    * `claimable_balance_sponsorship_removed` with the following fields:
        * `former_sponsor` - former sponsor of a claimable balance.
    * `signer_sponsorship_created` with the following fields:
        * `signer` - signer being sponsored.
        * `sponsor` - signer sponsor.
    * `signer_sponsorship_updated` with the following fields:
        * `signer` - signer being sponsored.
        * `former_sponsor` - the former sponsor of the signer.
        * `new_sponsor` - the new sponsor of the signer.
    * `signer_sponsorship_removed` with the following fields:
        * `former_sponsor` - former sponsor of a signer.

### Breaking

- Update `stellar-base` to `v4.0.0` which introduces a breaking change in the internal XDR library.

The following functions were renamed:

- `xdr.OperationBody.setOption()` -> `xdr.OperationBody.setOptions()`
- `xdr.OperationBody.manageDatum()` -> `xdr.OperationBody.manageData()`
- `xdr.OperationType.setOption()` -> `xdr.OperationType.setOptions()`
- `xdr.OperationType.manageDatum()` -> `xdr.OperationType.manageData()`

The following enum values were renamed in `OperationType`:

- `setOption` -> `setOptions`
- `manageDatum` -> `manageData`


## [v5.0.4](https://github.com/stellar/js-stellar-sdk/compare/v5.0.3...v5.0.4)

### Update
- Add `tx_set_operation_count` to `ledger` resource ([#559](https://github.com/stellar/js-stellar-sdk/pull/559)).

## [v5.0.3](https://github.com/stellar/js-stellar-sdk/compare/v5.0.2...v5.0.3)

### Fix
- Fix regression on `server.offer().forAccount()` which wasn't allowing streaming ([#533](https://github.com/stellar/js-stellar-sdk/pull/553)).

## [v5.0.2](https://github.com/stellar/js-stellar-sdk/compare/v5.0.1...v5.0.2)

### Update

- Allow submitTransaction to receive a FeeBumpTransaction ([#548](https://github.com/stellar/js-stellar-sdk/pull/548)).

## [v5.0.1](https://github.com/stellar/js-stellar-sdk/compare/v5.0.0...v5.0.1)

### Update

- Skip SEP0029 (memo required check) for multiplexed accounts ([#538](https://github.com/stellar/js-stellar-sdk/pull/538)).

### Fix
- Fix missing documentation for `stellar-base` ([#544](https://github.com/stellar/js-stellar-sdk/pull/544)).
- Move dom-monkeypatch to root types and publish to npm ([#543](https://github.com/stellar/js-stellar-sdk/pull/543)).

## [v5.0.0](https://github.com/stellar/js-stellar-sdk/compare/v4.1.0...v5.0.0)

### Add
- Add fee bump related attributes to `TransactionResponse` ([#532](https://github.com/stellar/js-stellar-sdk/pull/532)):
    - `fee_account: string`.
    - `fee_bump_transaction: FeeBumpTransactionResponse`:
      ```js
      interface FeeBumpTransactionResponse {
        hash: string;
        signatures: string[];
      }
      ```
    - `inner_transaction: InnerTransactionResponse`:
      ```js
      interface InnerTransactionResponse {
        hash: string;
        signatures: string[];
        max_fee: string;
      }
      ```
- Add `memo_bytes: string` to `TransactionResponse` ([#532](https://github.com/stellar/js-stellar-sdk/pull/532)).
- Add `authorize_to_maintain_liabilities: boolean` to `AllowTrustOperation` ([#532](https://github.com/stellar/js-stellar-sdk/pull/532)).
- Add `is_authorized_to_maintain_liabilities: boolean` to `BalanceLineNative` ([#532](https://github.com/stellar/js-stellar-sdk/pull/532)).
- Add new result codes to `TransactionFailedResultCodes` ([#531](https://github.com/stellar/js-stellar-sdk/pull/531)).
  ```js
  TX_FEE_BUMP_INNER_SUCCESS = "tx_fee_bump_inner_success",
  TX_FEE_BUMP_INNER_FAILED = "tx_fee_bump_inner_failed",
  TX_NOT_SUPPORTED = "tx_not_supported",
  TX_SUCCESS = "tx_success",
  TX_TOO_EARLY = "tx_too_early",
  TX_TOO_LATE = "tx_too_late",
  TX_MISSING_OPERATION = "tx_missing_operation",
  TX_INSUFFICIENT_BALANCE = "tx_insufficient_balance",
  TX_NO_SOURCE_ACCOUNT = "tx_no_source_account",
  TX_INSUFFICIENT_FEE = "tx_insufficient_fee",
  TX_INTERNAL_ERROR = "tx_internal_error",
  ```

### Breaking changes
- The attributes `max_fee` and `fee_charged` in `TransactionResponse` can be now a `number` or a `string`.
  Update your code to handle both types since Horizon will start sending `string` in version `1.3.0` ([#528](https://github.com/stellar/js-stellar-sdk/pull/528)).
- Bump `stellar-base` to `v3.0.0`: This new version of stellar-base brings support for protocol 13, including multiple breaking changes which might affect your code, please review the list of breaking changes in [stellar-base@3.0.0](https://github.com/stellar/js-stellar-base/releases/tag/v3.0.0) release ([#524](https://github.com/stellar/js-stellar-sdk/pull/524)).
- Make `networkPassphrase` a required argument in `Utils.buildChallengeTx` and  `Utils.readChallengeTx` ([#524](https://github.com/stellar/js-stellar-sdk/pull/524)).
- Remove `Server.paths` ([#525](https://github.com/stellar/js-stellar-sdk/pull/525)).

## [v5.0.0-alpha.2](https://github.com/stellar/js-stellar-sdk/compare/v5.0.0-alpha.1..v5.0.0-alpha.2)

### Update
- Update `stellar-base` to `v3.0.0-alpha-1`.

## [v5.0.0-alpha.1](https://github.com/stellar/js-stellar-sdk/compare/v4.1.0...v5.0.0-alpha.1)

### Breaking changes
- Bump `stellar-base` to `v3.0.0-alpha-0`: This new version of stellar-base brings support for protocol 13, including multiple breaking changes which might affect your code, please review the list of breaking changes in [stellar-base@3.0.0-alpha.0](https://github.com/stellar/js-stellar-base/releases/tag/v3.0.0-alpha.0) release ([#524](https://github.com/stellar/js-stellar-sdk/pull/524)).
- Make `networkPassphrase` a required argument in `Utils.buildChallengeTx` and  `Utils.readChallengeTx` ([#524](https://github.com/stellar/js-stellar-sdk/pull/524)).
- Remove `Server.paths` ([#525](https://github.com/stellar/js-stellar-sdk/pull/525)).

## [v4.1.0](https://github.com/stellar/js-stellar-sdk/compare/v4.0.2...v4.1.0)

### Add
- Add SEP0029 (memo required) support. ([#516](https://github.com/stellar/js-stellar-sdk/issues/516))

  Extends `server.submitTransaction` to always run a memo required check before
  sending the transaction.  If any of the destinations require a memo and the
  transaction doesn't include one, then an `AccountRequiresMemoError` will be thrown.

  You can skip this check by passing `{skipMemoRequiredCheck: true}` to `server.submitTransaction`:

  ```
  server.submitTransaction(tx, {skipMemoRequiredCheck: true})
  ```

  The check runs for each operation of type:
   - `payment`
   - `pathPaymentStrictReceive`
   - `pathPaymentStrictSend`
   - `mergeAccount`

  If the transaction includes a memo, then memo required checking is skipped.

  See [SEP0029](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0029.md) for more information about memo required check.

## [v4.0.2](https://github.com/stellar/js-stellar-sdk/compare/v4.0.1...v4.0.2)

### Fix
- Fix URI TypeScript reference. ([#509](https://github.com/stellar/js-stellar-sdk/issues/509))
- Fix docs build. ([#503](https://github.com/stellar/js-stellar-sdk/issues/503))
- Fix documentation for method to filter offers by account. ([#507](https://github.com/stellar/js-stellar-sdk/issues/507))
- Fix types and add missing attribute to `account_response`. ([#504](https://github.com/stellar/js-stellar-sdk/issues/504))

## [v4.0.1](https://github.com/stellar/js-stellar-sdk/compare/v4.0.0...v4.0.1)

### Add
- Add `.offer` method to `OfferCallBuilder` which allows fetching a single offer by ID. ([#499](https://github.com/stellar/js-stellar-sdk/issues/499))

### Fix
- Fix broken link to Stellar logo+wordmark. ([#496](https://github.com/stellar/js-stellar-sdk/issues/496))
- Fix `_link` omition for AccountResponse class. ([#495](https://github.com/stellar/js-stellar-sdk/issues/495))

### Update
- Update challenge transaction helpers for SEP0010. ([#497](https://github.com/stellar/js-stellar-sdk/issues/497))

## [v4.0.0](https://github.com/stellar/js-stellar-sdk/compare/v3.3.0...v4.0.0)

### Added
- Add support for top-level offers endpoint with `seller`, `selling`, and `buying` filter. ([#485](https://github.com/stellar/js-stellar-sdk/issues/485))
  Horizon 1.0 includes a new `/offers` end-point, which allows you to list all offers, supporting filtering by `seller`, `selling`, or `buying` asset.

  You can fetch data from this endpoint by doing `server.offers()` and use any of the following filters:

  - `seller`: `server.offers().forAccount(accountId)`
  - `buying`: `server.offers().buying(asset)`
  - `selling`: `server.offers().selling(asset)`

  This introduced a breaking change since it modified the signature for the function `server.offers()`.

  Before, if you wanted to list all the offers for a given account, you'd do:

  ```
  server.offers('accounts', accountID)
  ```

  Starting on this version you'll need to do:

  ```
  server.offers().forAccount(accountId)
  ```

  You can do now things that were not possible before, like finding
  all offers for an account filtering by the selling or buying asset

  ```
  server.offers().forAccount(accountId).selling(assetA).buying(assetB)
  ```

- Add support for filtering accounts by `signer` or `asset` ([#474](https://github.com/stellar/js-stellar-sdk/issues/474))
  Horizon 1.0 includes a new `/accounts` end-point, which allows you to list all accounts who have another account as a signer or hold a given asset.

  You can fetch data from this endpoint by doing `server.accounts()` and use any of the following filters:

  - `accountID`: `server.accounts().accountId(accountId)`, returns a single account.
  - `forSigner`: `server.accounts().forSigner(accountId)`, returns accounts where `accountId` is a signer.
  - `forAsset`: `server.accounts().forAsset(asset)`, returns accounts which hold the `asset`.

- Add TypeScript typings for new fields in `fee_stats`. ([#462](https://github.com/stellar/js-stellar-sdk/issues/462))


### Changed
- Changed TypeScript typing for multiple operations "type", it will match the new value on Horizon. ([#477](https://github.com/stellar/js-stellar-sdk/issues/477))

### Fixed
- Fix fetchTimebounds() ([#487](https://github.com/stellar/js-stellar-sdk/issues/487))
- Clone the passed URI in CallBuilder constructor, to not mutate the outside ref ([#473](https://github.com/stellar/js-stellar-sdk/issues/473))
- Use axios CancelToken to ensure timeout ([#482](https://github.com/stellar/js-stellar-sdk/issues/482))

### Breaking
- Remove `fee_paid` field from transaction response. ([#476](https://github.com/stellar/js-stellar-sdk/issues/476))
- Remove all `*_accepted_fee` from FeeStatsResponse. ([#463](https://github.com/stellar/js-stellar-sdk/issues/463))
- Change function signature for `server.offers`. ([#485](https://github.com/stellar/js-stellar-sdk/issues/485))
  The signature for the function `server.offers()` was changed to bring suppport for other filters.

  Before, if you wanted to list all the offers for a given account, you'd do:

  ```
  server.offers('accounts', accountID)
  ```

  Starting on this version you'll need to do:

  ```
  server.offers().accountId(accountId)
  ```


## [v3.3.0](https://github.com/stellar/js-stellar-sdk/compare/v3.2.0...v3.3.0)

### Deprecated ⚠️

- Horizon 0.25.0 will change the data type for multiple attributes from `Int64` to
  `string`. When the JSON payload includes an `Int64`, there are
  scenarios where large number data can be incorrectly parsed, since JavaScript doesn't support
  `Int64` values. You can read more about it in [#1363](https://github.com/stellar/go/issues/1363).

  This release extends the data types for the following attributes to be of type `string` or `number`:

  - `EffectRecord#offer_id`
  - `EffectRecord#new_seq`
  - `OfferRecord#id`
  - `TradeAggregationRecord#timestamp`
  - `TradeAggregationRecord#trade_count`
  - `ManageOfferOperationResponse#offer_id`
  - `PassiveOfferOperationResponse#offer_id`

  We recommend you update your code to handle both `string` or `number` in
  the fields listed above, so that once Horizon 0.25.0 is released, your application
  will be able to handle the new type without breaking.

## [v3.2.0](https://github.com/stellar/js-stellar-sdk/compare/v3.1.2...v3.2.0)

### Add ➕

- Add `fee_charged` an `max_fee` to `TransactionResponse` interface. ([455](https://github.com/stellar/js-stellar-sdk/pull/455))

### Deprecated ⚠️

- Horizon 0.25 will stop sending the property `fee_paid` in the transaction response. Use `fee_charged` and `max_fee`, read more about it in [450](https://github.com/stellar/js-stellar-sdk/issues/450).

## [v3.1.2](https://github.com/stellar/js-stellar-sdk/compare/v3.1.1...v3.1.2)

### Change

- Upgrade `stellar-base` to `v2.1.2`. ([452](https://github.com/stellar/js-stellar-sdk/pull/452))

## [v3.1.1](https://github.com/stellar/js-stellar-sdk/compare/v3.1.0...v3.1.1)

### Change ⚠️

- Change arguments on [server.strictReceivePaths](https://stellar.github.io/js-stellar-sdk/Server.html#strictReceivePaths) since we included `destinationAccount` as an argument, but it is not longer required by Horizon. ([477](https://github.com/stellar/js-stellar-sdk/pull/447))

## [v3.1.0](https://github.com/stellar/js-stellar-sdk/compare/v3.0.0...v3.1.0)

### Add ➕

- Add `server.strictReceivePaths` which adds support for `/paths/strict-receive`. ([444](https://github.com/stellar/js-stellar-sdk/pull/444))
  This function takes a list of source assets or a source address, a destination
  address, a destination asset and a destination amount.

  You can call it passing a list of source assets:

  ```
  server.strictReceivePaths(sourceAssets,destinationAsset, destinationAmount)
  ```

  Or a by passing a Stellar source account address:

  ```
  server.strictReceivePaths(sourceAccount,destinationAsset, destinationAmount)
  ```

  When you call this function with a Stellar account address, it will look at the account’s trustlines and use them to determine all payment paths that can satisfy the desired amount.

- Add `server.strictSendPaths` which adds support for `/paths/strict-send`. ([444](https://github.com/stellar/js-stellar-sdk/pull/444))
  This function takes the asset you want to send, and the amount of that asset,
  along with either a list of destination assets or a destination address.

  You can call it passing a list of destination assets:

  ```
  server.strictSendPaths(sourceAsset, sourceAmount, [destinationAsset]).call()
  ```

  Or a by passing a Stellar account address:

  ```
  server.strictSendPaths(sourceAsset, sourceAmount, "GDRREYWHQWJDICNH4SAH4TT2JRBYRPTDYIMLK4UWBDT3X3ZVVYT6I4UQ").call()
  ```

  When you call this function with a Stellar account address, it will look at the account’s trustlines and use them to determine all payment paths that can satisfy the desired amount.

### Deprecated ⚠️

- [Server#paths](https://stellar.github.io/js-stellar-sdk/Server.html#paths) is deprecated in favor of [Server#strictReceivePaths](https://stellar.github.io/js-stellar-sdk/Server.html#strictReceivePaths). ([444](https://github.com/stellar/js-stellar-sdk/pull/444))

## [v3.0.1](https://github.com/stellar/js-stellar-sdk/compare/v3.0.0...v3.0.1)

### Add
- Add join method to call builder. ([#436](https://github.com/stellar/js-stellar-sdk/issues/436))

## [v3.0.0](https://github.com/stellar/js-stellar-sdk/compare/v2.3.0...v3.0.0)

### BREAKING CHANGES ⚠

- Drop Support for Node 6 since it has been end-of-lifed and no longer in LTS. We now require Node 10 which is the current LTS until April 1st, 2021. ([#424](https://github.com/stellar/js-stellar-sdk/pull/424)

## [v2.3.0](https://github.com/stellar/js-stellar-sdk/compare/v2.2.3...v2.3.0)

### Add
- Add feeStats support. ([#409](https://github.com/stellar/js-stellar-sdk/issues/409))

### Fix
- Fix Util.verifyChallengeTx documentation ([#405](https://github.com/stellar/js-stellar-sdk/issues/405))
- Fix: listen to stream events with addEventListener ([#408](https://github.com/stellar/js-stellar-sdk/issues/408))

## [v2.2.3](https://github.com/stellar/js-stellar-sdk/compare/v2.2.2...v2.2.3)

### Fix
- Fix ServerApi's OrderbookRecord type ([#401](https://github.com/stellar/js-stellar-sdk/issues/401))

### Set
- Set `name` in custom errors ([#403](https://github.com/stellar/js-stellar-sdk/issues/403))

## [v2.2.2](https://github.com/stellar/js-stellar-sdk/compare/v2.2.1...v2.2.2)

### Fix

- Fix manage data value in SEP0010 challenge builder. ([#396](https://github.com/stellar/js-stellar-sdk/issues/396))

### Add

- Add support for networkPassphrase in SEP0010 challenge builder. ([#397](https://github.com/stellar/js-stellar-sdk/issues/397))

## [v2.2.1](https://github.com/stellar/js-stellar-sdk/compare/v2.2.0...v2.2.1)

### Fix

- Fix [#391](https://github.com/stellar/js-stellar-sdk/issues/391): Remove instance check for MessageEvent on stream error. ([#392](https://github.com/stellar/js-stellar-sdk/issues/392))


## [v2.2.0](https://github.com/stellar/js-stellar-sdk/compare/v2.1.1...v2.2.0)

### Add
- Add helper `Utils.verifyChallengeTx` to verify SEP0010 "Challenge" Transaction. ([#388](https://github.com/stellar/js-stellar-sdk/issues/388))
- Add helper `Utils.verifyTxSignedBy` to verify that a transaction has been signed by a given account. ([#388](https://github.com/stellar/js-stellar-sdk/pull/388/commits/2cbf36891e529f63867d46bcf321b5bb76acef50))

### Fix
- Check for a global EventSource before deciding what to use. This allows you to inject polyfills in other environments like react-native. ([#389](https://github.com/stellar/js-stellar-sdk/issues/389))

## [v2.1.1](https://github.com/stellar/js-stellar-sdk/compare/v2.1.0...v2.1.1)

### Fix
- Fix CallBuilder onmessage type ([#385](https://github.com/stellar/js-stellar-sdk/issues/385))

## [v2.1.0](https://github.com/stellar/js-stellar-sdk/compare/v2.0.1...v2.1.0)

### Add
- Add single script to build docs and call it when combined with jsdoc. ([#380](https://github.com/stellar/js-stellar-sdk/issues/380))
- Add SEP0010 transaction challenge builder. ([#375](https://github.com/stellar/js-stellar-sdk/issues/375))
- Add `home_domain` to ServerApi.AccountRecord ([#376](https://github.com/stellar/js-stellar-sdk/issues/376))

### Bump
- Bump stellar-base to 1.0.3. ([#378](https://github.com/stellar/js-stellar-sdk/issues/378))
- Bump @stellar/tslint-config ([#377](https://github.com/stellar/js-stellar-sdk/issues/377))

### Fix
- Fix jsdoc's build in after_deploy ([#373](https://github.com/stellar/js-stellar-sdk/issues/373))
- Create new URI instead of passing serverUrl (Fix [#379](https://github.com/stellar/js-stellar-sdk/issues/379)). ([#382](https://github.com/stellar/js-stellar-sdk/issues/382))

## [v2.0.1](https://github.com/stellar/js-stellar-sdk/compare/v1.0.2...v2.0.1)

- **Breaking change** Port stellar-sdk to Typescript. Because we use a slightly
  different build process, there could be some unanticipated bugs. Additionally,
  some type definitions have changed:
  - Types that were once in the `Server` namespace but didn't actually deal with
    the `Server` class have been broken out into a new namespace, `ServerApi`.
    So, for example, `Server.AccountRecord` -> `ServerApi.AccountRecord`.
  - `Server.AccountResponse` is out of the `Server` namespace ->
    `AccountResponse`
  - `Server.*CallBuilder` is out of the `Server` namespace -> `*CallBuilder`
  - `HorizonResponseAccount` is now `Horizon.AccountResponse`
- Upgrade Webpack to v4.
- Add support for providing app name and version to request headers.
- (NPM wouldn't accept the 2.0.0 version, so we're publishing to 2.0.1.)

Many thanks to @Ffloriel and @Akuukis for their help with this release!

## [v1.0.5](https://github.com/stellar/js-stellar-sdk/compare/v1.0.4...v1.0.5)

- Make CallCollectionFunction return a CollectionPage.
- Update Horizon.AccountSigner[] types.

## [v1.0.4](https://github.com/stellar/js-stellar-sdk/compare/v1.0.3...v1.0.4)

- Automatically tag alpha / beta releases as "next" in NPM.

## [v1.0.3](https://github.com/stellar/js-stellar-sdk/compare/v1.0.2...v1.0.3)

- Upgrade axios to 0.19.0 to close a security vulnerability.
- Some type fixes.

## [v1.0.2](https://github.com/stellar/js-stellar-sdk/compare/v1.0.1...v1.0.2)

- Upgrade stellar-base to v1.0.2 to fix a bug with the browser bundle.

## [v1.0.1](https://github.com/stellar/js-stellar-sdk/compare/v1.0.0...v1.0.1)

- Upgrade stellar-base to v1.0.1, which makes available again the deprecated
  operation functions `Operation.manageOffer` and `Operation.createPassiveOffer`
  (with a warning).
- Fix the documentation around timebounds.

## [v1.0.0](https://github.com/stellar/js-stellar-sdk/compare/v0.15.4...v1.0.0)

- Upgrade stellar-base to
  [v1.0.0](https://github.com/stellar/js-stellar-base/releases/tag/v1.0.0),
  which introduces two breaking changes.
- Switch stellar-sdk's versioning to true semver! 🎉

## [v0.15.4](https://github.com/stellar/js-stellar-sdk/compare/v0.15.3...v0.15.4)

- Add types for LedgerCallBuilder.ledger.
- Add types for Server.operationFeeStats.
- Add types for the HorizonAxiosClient export.
- Move @types/\* from devDependencies to dependencies.
- Pass and use a stream response type to CallBuilders if it's different from the
  normal call response.
- Upgrade stellar-base to a version that includes types, and remove
  @types/stellar-base as a result.

## [v0.15.3](https://github.com/stellar/js-stellar-sdk/compare/v0.15.2...v0.15.3)

- In .travis.yml, try to switch from the encrypted API key to an environment
  var.

## [v0.15.2](https://github.com/stellar/js-stellar-sdk/compare/v0.15.1...v0.15.2)

- Fix Server.transactions and Server.payments definitions to properly return
  collections
- Renew the npm publish key

## [v0.15.1](https://github.com/stellar/js-stellar-sdk/compare/v0.15.0...v0.15.1)

- Add Typescript type definitions (imported from DefinitelyTyped).
- Make these changes to those definitions:
  - Add definitions for Server.fetchBaseFee and Server.fetchTimebounds
  - CallBuilder: No long always returns CollectionPaged results. Interfaces that
    extend CallBuilder should specify whether their response is a collection or
    not
  - CallBuilder: Add inflation_destination and last_modified_ledger property
  - OfferRecord: Fix the returned properties
  - TradeRecord: Fix the returned properties
  - TradesCallBuilder: Add forAccount method
  - TransactionCallBuilder: Add includeFailed method
  - Horizon.BalanceLineNative/Asset: Add buying_liabilities /
    selling_liabilities properties
- Fix documentation links.

## [v0.15.0](https://github.com/stellar/js-stellar-sdk/compare/v0.14.0...v0.15.0)

- **Breaking change**: `stellar-sdk` no longer ships with an `EventSource`
  polyfill. If you plan to support IE11 / Edge, please use
  [`event-source-polyfill`](https://www.npmjs.com/package/event-source-polyfill)
  to set `window.EventSource`.
- Upgrade `stellar-base` to a version that doesn't use the `crypto` library,
  fixing a bug with Angular 6
- Add `Server.prototype.fetchTimebounds`, a helper function that helps you set
  the `timebounds` property when initting `TransactionBuilder`. It bases the
  timebounds on server time rather than local time.

## [v0.14.0](https://github.com/stellar/js-stellar-sdk/compare/v0.13.0...v0.14.0)

- Updated some out-of-date dependencies
- Update documentation to explicitly set fees
- Add `Server.prototype.fetchBaseFee`, which devs can use to fetch the current
  base fee; we plan to add more functions to help suggest fees in future
  releases
- Add `includeFailed` to `OperationCallBuilder` for including failed
  transactions in calls
- Add `operationFeeStats` to `Server` for the new fee stats endpoint
- After submitting a transaction with a `manageOffer` operation, return a new
  property `offerResults`, which explains what happened to the offer. See
  [`Server.prototype.submitTransaction`](https://stellar.github.io/js-stellar-sdk/Server.html#submitTransaction)
  for documentation.

## 0.13.0

- Update `stellar-base` to `0.11.0`
- Added ESLint and Prettier to enforce code style
- Upgraded dependencies, including Babel to 6
- Bump local node version to 6.14.0

## 0.12.0

- Update `stellar-base` to `0.10.0`:
  - **Breaking change** Added
    [`TransactionBuilder.setTimeout`](https://stellar.github.io/js-stellar-base/TransactionBuilder.html#setTimeout)
    method that sets `timebounds.max_time` on a transaction. Because of the
    distributed nature of the Stellar network it is possible that the status of
    your transaction will be determined after a long time if the network is
    highly congested. If you want to be sure to receive the status of the
    transaction within a given period you should set the TimeBounds with
    `maxTime` on the transaction (this is what `setTimeout` does internally; if
    there's `minTime` set but no `maxTime` it will be added). Call to
    `TransactionBuilder.setTimeout` is required if Transaction does not have
    `max_time` set. If you don't want to set timeout, use `TimeoutInfinite`. In
    general you should set `TimeoutInfinite` only in smart contracts. Please
    check
    [`TransactionBuilder.setTimeout`](https://stellar.github.io/js-stellar-base/TransactionBuilder.html#setTimeout)
    docs for more information.
  - Fixed decoding empty `homeDomain`.
- Add `offset` parameter to TradeAggregationCallBuilder to reflect new changes
  to the endpoint in horizon-0.15.0

## 0.11.0

- Update `js-xdr` (by updating `stellar-base`) to support unmarshaling non-utf8
  strings.
- String fields returned by `Operation.fromXDRObject()` are of type `Buffer` now
  (except `SetOptions.home_domain` and `ManageData.name` - both required to be
  ASCII by stellar-core).

## 0.10.3

- Update `stellar-base` and xdr files.

## 0.10.2

- Update `stellar-base` (and `js-xdr`).

## 0.10.1

- Update `stellar-base` to `0.8.1`.

## 0.10.0

- Update `stellar-base` to `0.8.0` with `bump_sequence` support.

## 0.9.2

- Removed `.babelrc` file from the NPM package.

## 0.9.1

### Breaking changes

- `stellar-sdk` is now using native `Promise` instead of `bluebird`. The `catch`
  function is different. Instead of:

  ```js
  .catch(StellarSdk.NotFoundError, function (err) { /* ... */ })
  ```

  please use the following snippet:

  ```js
  .catch(function (err) {
    if (err instanceof StellarSdk.NotFoundError) { /* ... */ }
  })
  ```

- We no longer support IE 11, Firefox < 42, Chrome < 49.

### Changes

- Fixed `_ is undefined` bug.
- Browser build is around 130 KB smaller!

## 0.8.2

- Added `timeout` option to `StellarTomlResolver` and `FederationServer` calls
  (https://github.com/stellar/js-stellar-sdk/issues/158).
- Fixed adding random value to URLs multiple times
  (https://github.com/stellar/js-stellar-sdk/issues/169).
- Fixed jsdoc for classes that extend `CallBuilder`.
- Updated dependencies.
- Added `yarn.lock` file to repository.

## 0.8.1

- Add an allowed trade aggregation resolution of one minute
- Various bug fixes
- Improved documentation

## 0.8.0

- Modify `/trades` endpoint to reflect changes in horizon.
- Add `/trade_aggregations` support.
- Add `/assets` support.

## 0.7.3

- Upgrade `stellar-base`.

## 0.7.2

- Allow hex string in setOptions signers.

## 0.7.1

- Upgrade `stellar-base`.

## 0.7.0

- Support for new signer types: `sha256Hash`, `preAuthTx`.
- `StrKey` helper class with `strkey` encoding related methods.
- Removed deprecated methods: `Keypair.isValidPublicKey` (use `StrKey`),
  `Keypair.isValidSecretKey` (use `StrKey`), `Keypair.fromSeed`, `Keypair.seed`,
  `Keypair.rawSeed`.
- **Breaking changes**:
  - `Network` must be explicitly selected. Previously testnet was a default
    network.
  - `Operation.setOptions()` method `signer` param changed.
  - `Keypair.fromAccountId()` renamed to `Keypair.fromPublicKey()`.
  - `Keypair.accountId()` renamed to `Keypair.publicKey()`.
  - Dropping support for `End-of-Life` node versions.

## 0.6.2

- Updated `stellar.toml` location

## 0.6.1

- `forUpdate` methods of call builders now accept strings and numbers.
- Create a copy of attribute in a response if there is a link with the same name
  (ex. `transaction.ledger`, `transaction._links.ledger`).

## 0.6.0

- **Breaking change** `CallBuilder.stream` now reconnects when no data was
  received for a long time. This is to prevent permanent disconnects (more in:
  [#76](https://github.com/stellar/js-stellar-sdk/pull/76)). Also, this method
  now returns `close` callback instead of `EventSource` object.
- **Breaking change** `Server.loadAccount` now returns the `AccountResponse`
  object.
- **Breaking change** Upgraded `stellar-base` to `0.6.0`. `ed25519` package is
  now an optional dependency. Check `StellarSdk.FastSigning` variable to check
  if `ed25519` package is available. More in README file.
- New `StellarTomlResolver` class that allows getting `stellar.toml` file for a
  domain.
- New `Config` class to set global config values.

## 0.5.1

- Fixed XDR decoding issue when using firefox

## 0.5.0

- **Breaking change** `Server` and `FederationServer` constructors no longer
  accept object in `serverUrl` parameter.
- **Breaking change** Removed `AccountCallBuilder.address` method. Use
  `AccountCallBuilder.accountId` instead.
- **Breaking change** It's no longer possible to connect to insecure server in
  `Server` or `FederationServer` unless `allowHttp` flag in `opts` is set.
- Updated dependencies.

## 0.4.3

- Updated dependency (`stellar-base`).

## 0.4.2

- Updated dependencies.
- Added tests.
- Added `CHANGELOG.md` file.

## 0.4.1

- `stellar-base` bump. (c90c68f)

## 0.4.0

- **Breaking change** Bumped `stellar-base` to
  [0.5.0](https://github.com/stellar/js-stellar-base/blob/master/CHANGELOG.md#050).
  (b810aef)
