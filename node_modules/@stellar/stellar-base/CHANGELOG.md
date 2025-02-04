# Changelog

## Unreleased


## [`v13.0.1`](https://github.com/stellar/js-stellar-base/compare/v13.0.0...v13.0.1)

### Fixed
* `buildInvocationTree` will now successfully walk creation invocations with constructor arguments ([#784](https://github.com/stellar/js-stellar-base/pull/784)).


## [`v13.0.0`](https://github.com/stellar/js-stellar-base/compare/v12.1.1...v13.0.0)

**This release supports Protocol 22.** While the network has not upgraded yet, you can start integrating the new features into your codebase if you want a head start. Keep in mind that while the binary XDR is backwards-compatible, the naming and layout of structures is not. In other words, this build will continue to work on Protocol 21, but you may have to update code that references XDR directly.

This version is unchanged from [`beta.1`](#v13.0.0-beta.1).


## [`v13.0.0-beta.1`](https://github.com/stellar/js-stellar-base/compare/v12.1.1...v13.0.0-beta.1)

**This is the first release that supports Protocol 22.** While the network has not upgraded yet, you can start integrating the new features into your codebase if you want a head start. Keep in mind that while the binary XDR is backwards-compatible, the naming and layout of structures is not. In other words, this build will continue to work on Protocol 21, but you may have to update code that references XDR directly.

### Breaking Changes
* XDR definitions have been upgraded to Protocol 22 ([#777](https://github.com/stellar/js-stellar-base/pull/777)).

### Added
* You can create contracts with constructors a new, optional parameter of `Operation.createCustomContract`, `constructorArgs: xdr.ScVal[]` ([#770](https://github.com/stellar/js-stellar-base/pull/770)).


## [`v12.1.1`](https://github.com/stellar/js-stellar-base/compare/v12.1.0...v12.1.1)

### Fixed
* Add missing methods to TypeScript definitions ([#766](https://github.com/stellar/js-stellar-base/pull/766)).
* Fix the TypeScript definition of `walkInvocationTree` to allow void returns on the callback function as intended rather than forcing a `return null` ([#765](https://github.com/stellar/js-stellar-base/pull/765)).
* Fix `authorizeEntry` to use the correct public key when passing `Keypair`s ([#772](https://github.com/stellar/js-stellar-base/pull/772)).
* Upgrade misc. dependencies ([#771](https://github.com/stellar/js-stellar-base/pull/771), [#773](https://github.com/stellar/js-stellar-base/pull/773)).


## [`v12.1.0`](https://github.com/stellar/js-stellar-base/compare/v12.0.1...v12.1.0)

### Added
* `TransactionBuilder` now has `addOperationAt` and `clearOperationAt` methods to allow manipulation of individual operations ([#757](https://github.com/stellar/js-stellar-base/pull/757)).

### Fixed
* Improve the efficiency and portability of asset type retrieval ([#758](https://github.com/stellar/js-stellar-base/pull/758)).
* `nativeToScVal` now correctly sorts maps lexicographically based on the keys to match what the Soroban environment expects ([#759](https://github.com/stellar/js-stellar-base/pull/759)).
* `nativeToScVal` now allows all integer types to come from strings ([#763](https://github.com/stellar/js-stellar-base/pull/763)).
* `humanizeEvents` now handles events without a `contractId` set more reliably ([#764](https://github.com/stellar/js-stellar-base/pull/764)).


## [`v12.0.1`](https://github.com/stellar/js-stellar-base/compare/v12.0.0...v12.0.1)

### Fixed
* Export TypeScript definition for `StrKey.isValidContract` ([#751](https://github.com/stellar/js-stellar-base/pull/751)).
* `scValToNative` would fail when the values contained error codes because the parsing routine hadn't been updated to the new error schemas. Errors are now converted to the following format ([#753](https://github.com/stellar/js-stellar-base/pull/753)):

```typescript
interface Error {
  type: "contract" | "system";
  code: number;
  value?: string; // only present for type === 'system'
}
```

You can refer to the [XDR documentation](https://github.com/stellar/stellar-xdr/blob/70180d5e8d9caee9e8645ed8a38c36a8cf403cd9/Stellar-contract.x#L76-L115) for additional explanations for each error code.


## [`v12.0.0`](https://github.com/stellar/js-stellar-base/compare/v11.0.1...v12.0.0)

This is a re-tag of v12.0.0-rc.1 with only developer dependency updates in-between.


## [`v12.0.0-rc.1`](https://github.com/stellar/js-stellar-base/compare/v11.0.1...v12.0.0-rc.1)

### Breaking Changes
* The generated XDR has been upgraded to match the upcoming Protocol 21, namely [stellar/stellar-xdr@`1a04392`](https://github.com/stellar/stellar-xdr/commit/1a04392432dacc0092caaeae22a600ea1af3c6a5) ([#738](https://github.com/stellar/js-stellar-base/pull/738)).

### Added
* To facilitate serialization and deserialization for downstream systems, this package now exports `cereal.XdrWriter` and `cereal.XdrReader` which come directly from `@stellar/js-xdr` ([#744](https://github.com/stellar/js-stellar-base/pull/744)).

### Fixed
* Updated various dependencies ([#737](https://github.com/stellar/js-stellar-base/pull/737), [#739](https://github.com/stellar/js-stellar-base/pull/739)).
* `Buffer` and `Uint8Array` compatibility has improved in `StrKey` ([#746](https://github.com/stellar/js-stellar-base/pull/746)).


## [`v11.0.1`](https://github.com/stellar/js-stellar-base/compare/v11.0.0...v11.0.1)

### Fixed
* Add compatibility with pre-ES2016 environments (like some React Native JS compilers) by adding a custom `Buffer.subarray` polyfill ([#733](https://github.com/stellar/js-stellar-base/pull/733)).
* Upgrade underlying dependencies, including `@stellar/js-xdr` which should broaden compatibility to pre-ES2016 environments ([#734](https://github.com/stellar/js-stellar-base/pull/734), [#735](https://github.com/stellar/js-stellar-base/pull/735)).


## [`v11.0.0`](https://github.com/stellar/js-stellar-base/compare/v10.0.2...v11.0.0)

**Note:** This version is (still) compatible with Protocol 20. Most people should be unaffected by the technically-breaking changes below and can treat this more like a v10.0.3 patch release.

### Breaking Changes
* Starting from **v10.0.0-beta.0**, we set [`BigNumber.DEBUG`](https://mikemcl.github.io/bignumber.js/#debug) in `bignumber.js` to `true` internally, which affects all code using `BigNumber`. This behavior has been fixed and only affects this library: globally, `BigNumber.DEBUG` now remains at its default setting (i.e. disabled). This is technically a breaking behavior change and is released as such ([#729](https://github.com/stellar/js-stellar-base/pull/729)).

### Fixed
* Dependencies have been updated to their latest compatible versions ([#726](https://github.com/stellar/js-stellar-base/pull/729), [#730](https://github.com/stellar/js-stellar-base/pull/730)).


## [`v10.0.2`](https://github.com/stellar/js-stellar-base/compare/v10.0.1...v10.0.2)

### Fixed
* The `contractId` field is correctly omitted from humanized events when it wasn't present in the structure ([#721](https://github.com/stellar/js-stellar-base/pull/721)).
* Misc. outdated or incorrect documentation has been updated ([#723](https://github.com/stellar/js-stellar-base/pull/723), [#720](https://github.com/stellar/js-stellar-base/pull/720)).
* Dependencies have been updated ([#724](https://github.com/stellar/js-stellar-base/pull/724)).


## [`v10.0.1`](https://github.com/stellar/js-stellar-base/compare/v10.0.0...v10.0.1)

### Fixed
* The TypeScript definition for `Asset.contractId()` now includes a missing parameter (the `networkPassphrase` changes the ID for a contract; [#718](https://github.com/stellar/js-stellar-base/pull/#718)).


## [`v10.0.0`](https://github.com/stellar/js-stellar-base/compare/v9.0.0...v10.0.0): Protocol 20 Stable Release

### Breaking Changes
* The new minimum supported Node version is Node 18.
* XDR has been upgraded to the latest stable version ([stellar-xdr@`6a620d1`](https://github.com/stellar/stellar-xdr/tree/6a620d160aab22609c982d54578ff6a63bfcdc01)). This is mostly renames, but it includes the following relevant breaking changes ([#704](https://github.com/stellar/js-stellar-base/pull/704)):
  - `Operation.bumpFootprintExpiration` is now `extendFootprintTtl` and its `ledgersToExpire` field is now named `extendTo`, but it serves the same purpose.
  - In TypeScript, the `Operation.BumpFootprintExpiration` is now `Operation.ExtendFootprintTTL`
  - `xdr.ContractExecutable.contractExecutableToken` is now `contractExecutableStellarAsset`
  - `xdr.SorobanTransactionData.refundableFee` is now `resourceFee`
  - In turn, `SorobanDataBuilder.setRefundableFee` is now `setResourceFee`
  - This new fee encompasses the entirety of the Soroban-related resource fees. Note that this is distinct from the "network-inclusion" fee that you would set on your transaction (i.e. `TransactionBuilder(..., { fee: ... })`).
- `Contract.getFootprint()` now only returns a single result: the ledger key of the deployed instance for the given ID, because the key for the code entry was incorrect (it should not be the ID but rather the WASM hash, which is not calculatable w/o network access) ([#709](https://github.com/stellar/js-stellar-base/pull/709)).


## [`v10.0.0-beta.4`](https://github.com/stellar/js-stellar-base/compare/v10.0.0-beta.3...v10.0.0-beta.4)

### Fixed
- You can now correctly clone transactions (`TransactionBuilder.cloneFrom`) with large sequence numbers ([#711](https://github.com/stellar/js-stellar-base/pull/711)).


## [`v10.0.0-beta.3`](https://github.com/stellar/js-stellar-base/compare/v10.0.0-beta.2...v10.0.0-beta.3)

### Fixed
* Fixes a bug where `authorizeEntry` might perform a no-op when it shouldn't ([#701](https://github.com/stellar/js-stellar-base/pull/701)).
* Fixes a TypeScript bug where `Memo.hash` did not accept a `Buffer` ([#698](https://github.com/stellar/js-stellar-base/pull/698)).
* Upgrades a transient dependency for security ([#296](https://github.com/stellar/js-stellar-base/pull/696)).


## [`v10.0.0-beta.2`](https://github.com/stellar/js-stellar-base/compare/v10.0.0-beta.1...v10.0.0-beta.2)

### Breaking Changes
 * The wrappers around multi-party authorization have changed ([#678](https://github.com/stellar/js-stellar-base/pull/678)):
  - `authorizeEntry` has been added to help sign auth entries in-place
  - the signature for `authorizeInvocation` has changed: it now offers a callback approach by default and requires slightly different parameters
  - `buildAuthEntry`, `buildAuthEnvelope`, and `authorizeInvocationCallback` have been removed

### Fixed
 * The TypeScript definitions for XDR schemas now point to the current protocol rather than vNext ([#694](https://github.com/stellar/js-stellar-base/pull/694)).
 * Misc. dependencies have been updated to their latest versions ([#694](https://github.com/stellar/js-stellar-base/pull/694)).


## [`v10.0.0-beta.1`](https://github.com/stellar/js-stellar-base/compare/v10.0.0-beta.0...v10.0.0-beta.1)

### Fixed
 * `nativeToScVal` now allows anything to be passed to the `opts.type` specifier. Previously, it was only integer types ([#691](https://github.com/stellar/js-stellar-base/pull/691)).
 * `Contract.call()` now produces valid `Operation` XDR ([#692](https://github.com/stellar/js-stellar-base/pull/692)).


## [`v10.0.0-beta.0`](https://github.com/stellar/js-stellar-base/compare/v9.0.0...v10.0.0-beta.0): Protocol 20

### Breaking Changes
 * **Node 16 is the new minimum version** to use the SDKs.
 * The XDR has been massively overhauled to support [Soroban in Protocol 20](https://soroban.stellar.org/docs/category/fundamentals-and-concepts), which means new operations, data structures, and a transaction format as well as new overlay features ([#538](https://github.com/stellar/js-stellar-base/pull/538)).

The core data structure of Soroban is a generic type called an `ScVal` (**s**mart **c**ontract **val**ue, which is a union of types that can basically represent anything [numbers, strings, arrays, maps, contract bytecode, etc.]). You can refer to the XDR for details, and you can utilize new APIs to make dealing with these complex values easier:
 - `nativeToScVal` helps convert native types to their closest Soroban equivalent
 - `scValToNative` helps find the closest native JavaScript type(s) corresponding to a smart contract value
 - `scValToBigInt` helps convert numeric `ScVal`s into native `bigint`s
 - `ScInt` and `XdrLargeInt` help convert to and from `bigint`s to other types and form sized integer types for smart contract usage

### Added
The following are new APIs to deal with new Soroban constructs:
 - **`Address`, which helps manage "smart" addresses in the Soroban context.** Addresses there (used for auth and identity purposes) can either be contracts (strkey `C...`) or accounts (strkey `G...`). This abstraction helps manage them and distinguish between them easily.
 - **`Contract`, which helps manage contract identifiers.** The primary purpose is to build invocations of its methods via the generic `call(...)`, but it also provides utilities for converting to an `Address` or calculating its minimum footprint for state expiration.
 - **Three new operations** have been added related to Soroban transactions:
   * `invokeHostFunction` for calling contract code
   * `bumpFootprintExpiration` for extending the state lifetime of Soroban data
   * `restoreFootprint` for restoring expired, off-chain state back onto the ledger
 - The `TransactionBuilder` now takes a `sorobanData` parameter (and has a corresponding `.setSorobanData()` builder method) which primarily describes the storage footprint of a Soroban (that is, which parts of the ledger state [in the form of `xdr.LedgerKey`s] it plans to read and write as part of the transaction).
   * To facilitate building this out, there's a new `SorobanDataBuilder` factory to set fields individually
 - The `TransactionBuilder` now has a `cloneFrom(tx, opts)` constructor method to create an instance from an existing transaction, also allowing parameter overrides via `opts`.
 - The following are convenience methods for building out certain types of smart contract-related structures:
   * `buildInvocationTree` and `walkInvocationTree` are both ways to visualize invocation calling trees better
   * `authorizeInvocation` helps multiple parties sign invocation calling trees
   * `humanizeEvents` helps make diagnostic events more readable
 - We've added a GHA to track bundle size changes as PRs are made. This protocol upgrade adds +18% to the final, minified bundle size which is significant but acceptable given the size of the upgrade.

### Fixes
* Improves the error messages when passing invalid amounts to deposit and withdraw operations ([#679](https://github.com/stellar/js-stellar-base/pull/679)).


## [v9.0.0](https://github.com/stellar/js-stellar-base/compare/v8.2.2..v9.0.0)

This is a large update and the following changelog incorporates ALL changes across the `beta.N` versions of this upgrade.

This version is marked by a major version bump because of the significant upgrades to underlying dependencies. While there should be no noticeable API changes from a downstream perspective, there may be breaking changes in the way that this library is bundled.

The browser bundle size has decreased **significantly**:

  * `stellar-base.min.js` is **340 KiB**, down from **1.2 MiB** previously.
  * the new, unminified `stellar-base.js` is **895 KiB**.

### Breaking Changes

- The build system has been completely overhauled to support Webpack 5 ([#584](https://github.com/stellar/js-stellar-base/pull/584), [#585](https://github.com/stellar/js-stellar-base/pull/585)).

Though we have tried to maintain compatibility with older JavaScript implementations, this still means you may need to update your build pipeline to transpile to certain targets.

### Fixes

- Fixes a bug when sorting mixed-case assets for liquidity pools ([#606](https://github.com/stellar/js-stellar-base/pull/606)).
- Documentation is fixed and should generate correctly on https://stellar.github.io/js-stellar-base/ ([#609](https://github.com/stellar/js-stellar-base/pull/609)).

### Updates

- XDR has been updated to its latest version (both `curr` and `next` versions, [#587](https://github.com/stellar/js-stellar-base/pull/587)).
- Drop the `lodash` dependency entirely ([#624](https://github.com/stellar/js-stellar-base/issues/624)).
- Drop the `crc` dependency and inline it to lower bundle size ([#621](https://github.com/stellar/js-stellar-base/pull/621)).
- Upgrade all dependencies to their latest versions ([#608](https://github.com/stellar/js-stellar-base/pull/608)).


## [v9.0.0-beta.3](https://github.com/stellar/js-stellar-base/compare/v9.0.0-beta.1..v9.0.0-beta.2)

### Fix

- Fixes a bug when sorting mixed-case assets for liquidity pools ([#606](https://github.com/stellar/js-stellar-base/pull/606)).

### Update
- Upgrade all dependencies to their latest versions ([#608](https://github.com/stellar/js-stellar-base/pull/608)).
- Drop the `crc` dependency and inline it to lower bundle size ([#621](https://github.com/stellar/js-stellar-base/pull/621)).


## [v9.0.0-beta.2](https://github.com/stellar/js-stellar-base/compare/v9.0.0-beta.1..v9.0.0-beta.2)

### Update

- Upgrades the `js-xdr` dependency (major performance improvements, see [`js-xdr@v2.0.0`](https://github.com/stellar/js-xdr/releases/tag/v2.0.0)) and other dependencies to their latest versions ([#592](https://github.com/stellar/js-stellar-base/pull/592)).


## [v9.0.0-beta.1](https://github.com/stellar/js-stellar-base/compare/v9.0.0-beta.0..v9.0.0-beta.1)

### Fix

- Correct XDR type definition for raw `xdr.Operation`s ([#591](https://github.com/stellar/js-stellar-base/pull/591)).


## [v9.0.0-beta.0](https://github.com/stellar/js-stellar-base/compare/v8.2.2..v9.0.0-beta.0)

This version is marked by a major version bump because of the significant upgrades to underlying dependencies. While there should be no noticeable API changes from a downstream perspective, there may be breaking changes in the way that this library is bundled.

### Fix

- Build system has been overhauled to support Webpack 5 ([#585](https://github.com/stellar/js-stellar-base/pull/585)).

- Current and vNext XDR updated to latest versions ([#587](https://github.com/stellar/js-stellar-base/pull/587)).


## [v8.2.2](https://github.com/stellar/js-stellar-base/compare/v8.2.1..v8.2.2)

### Fix

- Enable signing in service workers using FastSigning ([#567](https://github.com/stellar/js-stellar-base/pull/567)).

## [v8.2.1](https://github.com/stellar/js-stellar-base/compare/v8.2.0..v8.2.1)

### Fix

* Turn all XLM-like (i.e. casing agnostic) asset codes into the native asset with code `XLM` ([#546](https://github.com/stellar/js-stellar-base/pull/546)).


## [v8.2.0](https://github.com/stellar/js-stellar-base/compare/v8.1.0..v8.2.0)

### Add

* `Operation.setOptions` now supports the new [CAP-40](https://stellar.org/protocol/cap-40) signed payload signer (`ed25519SignedPayload`) thanks to @orbitlens ([#542](https://github.com/stellar/js-stellar-base/pull/542)).


## [v8.1.0](https://github.com/stellar/js-stellar-base/compare/v8.0.1..v8.1.0)

### Add

* `TransactionBase.addDecoratedSignature` is a clearer way to add signatures directly to a built transaction without fiddling with the underlying `signatures` array ([#535](https://github.com/stellar/js-stellar-base/pull/535)).

* Update the XDR definitions (and the way in which they're generated) to contain both the latest current XDR (which introduces [CAP-42](https://stellar.org/protocol/cap-42)) and the "v-next" XDR (which contains XDR related to Soroban and should be considered unstable) ([#537](https://github.com/stellar/js-stellar-base/pull/537)).

### Fix

* Correctly set `minAccountSequence` in `TransactionBuilder` for large values ([#539](https://github.com/stellar/js-stellar-base/pull/539), thank you @overcat!).


## [v8.0.1](https://github.com/stellar/js-stellar-base/compare/v8.0.0..v8.0.1)

### Fix

- Correctly predict claimable balance IDs with large sequence numbers ([#530](https://github.com/stellar/js-stellar-base/pull/530), thank you @overcat!).


## [v8.0.0](https://github.com/stellar/js-stellar-base/compare/v7.0.0..v8.0.0)

This is a promotion from the beta version without changes, now that the CAP-21 and CAP-40 implementations have made it into [stellar/stellar-core#master](https://github.com/stellar/stellar-core/tree/master/).


## [v8.0.0-beta.0](https://github.com/stellar/js-stellar-base/compare/v7.0.0..v8.0.0-beta.0)

**This release adds support for Protocol 19**, which includes [CAP-21](https://stellar.org/protocol/cap-21) (new transaction preconditions) and [CAP-40](https://stellar.org/protocol/cap-40) (signed payload signers).

This is considered a beta release until the XDR for the Stellar protocol stabilizes and is officially released.

### Breaking

As of this release, the minimum supported version of NodeJS is **14.x**.

- Two XDR types have been renamed:
  * `xdr.OperationId` is now `xdr.HashIdPreimage`
  * `xdr.OperationIdId` is now `xdr.HashIdPreimageOperationId`

### Add

- Support for converting signed payloads ([CAP-40](https://stellar.org/protocol/cap-40)) to and from their StrKey (`P...`) representation ([#511](https://github.com/stellar/js-stellar-base/pull/511)):
  * `Keypair.signPayloadDecorated(data)`
  * `StrKey.encodeSignedPayload(buf)`
  * `StrKey.decodeSignedPayload(str)`
  * `StrKey.isValidSignedPayload(str)`

- Support for creating transactions with the new preconditions ([CAP-21](https://stellar.org/protocol/cap-21)) via `TransactionBuilder` ([#513](https://github.com/stellar/js-stellar-base/pull/513)).

- A way to convert between addresses (like `G...` and `P...`, i.e. the `StrKey` class) and their respective signer keys (i.e. `xdr.SignerKey`s), particularly for use in the new transaction preconditions ([#520](https://github.com/stellar/js-stellar-base/pull/520)):
  * `SignerKey.decodeAddress(address)`
  * `SignerKey.encodeSignerKey(address)`
  * `TransactionBuilder.setTimebounds(min, max)`
  * `TransactionBuilder.setLedgerbounds(min, max)`
  * `TransactionBuilder.setMinAccountSequence(seq)`
  * `TransactionBuilder.setMinAccountSequenceAge(age)`
  * `TransactionBuilder.setMinAccountSequenceLedgerGap(gap)`
  * `TransactionBuilder.setExtraSigners([signers])`

### Fix

- Correct a TypeScript definition on the `RevokeLiquidityPoolSponsorship` operation ([#522](https://github.com/stellar/js-stellar-base/pull/522)).

- Resolves a bug that incorrectly sorted `Asset`s with mixed-case asset codes (it preferred lowercase codes incorrectly) ([#516](https://github.com/stellar/js-stellar-base/pull/516)).

- Update developer dependencies:
  * `isparta`, `jsdoc`, and `underscore` ([#500](https://github.com/stellar/js-stellar-base/pull/500))
  * `ajv` ([#503](https://github.com/stellar/js-stellar-base/pull/503))
  * `karma` ([#505](https://github.com/stellar/js-stellar-base/pull/505))
  * `minimist` ([#514](https://github.com/stellar/js-stellar-base/pull/514))


## [v7.0.0](https://github.com/stellar/js-stellar-base/compare/v6.0.6..v7.0.0)

This release introduces **unconditional support for muxed accounts** ([#485](https://github.com/stellar/js-stellar-base/pull/485)).

### Breaking Changes

In [v5.2.0](https://github.com/stellar/js-stellar-base/releases/tag/v5.2.0), we introduced _opt-in_ support for muxed accounts, where you would need to explicitly pass a `true` flag if you wanted to interpret muxed account objects as muxed addresses (in the form `M...`, see [SEP-23](https://stellar.org/protocol/sep-23)). We stated that this would become the default in the future. That is now the case.

The following fields will now always support muxed properties:

  * `FeeBumpTransaction.feeSource`
  * `Transaction.sourceAccount`
  * `Operation.sourceAccount`
  * `Payment.destination`
  * `PathPaymentStrictReceive.destination`
  * `PathPaymentStrictSend.destination`
  * `AccountMerge.destination`
  * `Clawback.from`

The following functions had a `withMuxing` parameter removed:

  - `Operation.fromXDRObject`
  - `Transaction.constructor`
  - `FeeBumpTransaction.constructor`
  - `TransactionBuilder.fromXDR`
  - `TransactionBuilder.buildFeeBumpTransaction`

The following functions will no longer check the `opts` object for a `withMuxing` field:

  - `TransactionBuilder.constructor`
  - `Operation.setSourceAccount`

There are several other breaking changes:

  - `TransactionBuilder.enableMuxedAccounts()` is removed
  - `decodeAddressToMuxedAccount()` and `encodeMuxedAccountToAddress()` no longer accept a second boolean parameter
  - `Account.createSubaccount()` and `MuxedAccount.createSubaccount()` are removed ([#487](https://github.com/stellar/js-stellar-base/pull/487)). You should prefer to create them manually:

```js
  let mux1 = new MuxedAccount(someAccount, '1');

  // before:
  let mux2 = mux1.createSubaccount('2');

  // now:
  let mux2 = new MuxedAccount(mux1.baseAccount(), '2');
```


 - Introduced a new helper method to help convert from muxed account addresses to their underlying Stellar addresses ([#485](https://github.com/stellar/js-stellar-base/pull/485)):

```ts
function extractBaseAddess(address: string): string;
```

 - The following muxed account validation functions are now available from Typescript ([#483](https://github.com/stellar/js-stellar-base/pull/483/files)):

```typescript
namespace StrKey {
  function encodeMed25519PublicKey(data: Buffer): string;
  function decodeMed25519PublicKey(data: string): Buffer;
  function isValidMed25519PublicKey(publicKey: string): boolean;
}

function decodeAddressToMuxedAccount(address: string, supportMuxing: boolean): xdr.MuxedAccount;
function encodeMuxedAccountToAddress(account: xdr.MuxedAccount, supportMuxing: boolean): string;
function encodeMuxedAccount(gAddress: string, id: string): xdr.MuxedAccount;
```

- Added a helper function `Transaction.getClaimableBalanceId(int)` which lets you pre-determine the hex claimable balance ID of a `createClaimableBalance` operation prior to submission to the network ([#482](https://github.com/stellar/js-stellar-base/pull/482)).

### Fix

- Add `Buffer` as a parameter type option for the `Keypair` constructor in Typescript ([#484](https://github.com/stellar/js-stellar-base/pull/484)).


## [v6.0.6](https://github.com/stellar/js-stellar-base/compare/v6.0.5..v6.0.6)

### Fix

- Upgrades dependencies: `path-parse` (1.0.6 --> 1.0.7) and `jszip` (3.4.0 to 3.7.1) ([#450](https://github.com/stellar/js-stellar-base/pull/450), [#458](https://github.com/stellar/js-stellar-base/pull/458)).


## [v6.0.5](https://github.com/stellar/js-stellar-base/compare/v6.0.4..v6.0.5)

This version bump fixes a security vulnerability in a _developer_ dependency; **please upgrade as soon as possible!** You may be affected if you are working on this package in a developer capacity (i.e. you've cloned this repository) and have run `yarn` or `yarn install` any time on Oct 22nd, 2021.

Please refer to the [security advisory](https://github.com/advisories/GHSA-pjwm-rvh2-c87w) for details.


### Security Fix
- Pin `ua-parser-js` to a known safe version ([#477](https://github.com/stellar/js-stellar-base/pull/477)).


## [v6.0.4](https://github.com/stellar/js-stellar-base/compare/v6.0.3..v6.0.4)

### Fix
- Allow muxed accounts when decoding transactions via `TransactionBuilder.fromXDR()` ([#470](https://github.com/stellar/js-stellar-base/pull/470)).


## [v6.0.3](https://github.com/stellar/js-stellar-base/compare/v6.0.2..v6.0.3)

### Fix
- When creating a `Transaction`, forward the optional `withMuxing` flag along to its operations so that their properties are also decoded with the appropriate muxing state ([#469](https://github.com/stellar/js-stellar-base/pull/469)).


## [v6.0.2](https://github.com/stellar/js-stellar-base/compare/v6.0.1..v6.0.2)

### Fix
- Fix Typescript signatures for operations to universally allow setting the `withMuxing` flag ([#466](https://github.com/stellar/js-stellar-base/pull/466)).


## [v6.0.1](https://github.com/stellar/js-stellar-base/compare/v5.3.2..v6.0.1)

### Add

- Introduced new CAP-38 operations `LiquidityPoolDepositOp` and `LiquidityPoolWithdrawOp`.
- Introduced two new types of assets, `LiquidityPoolId` and `LiquidityPoolAsset`.

### Update

- The XDR definitions have been updated to support CAP-38.
- Extended `Operation` class with the `Operation.revokeLiquidityPoolSponsorship` helper that allows revoking a liquidity pool sponsorship.
- Asset types now include `AssetType.liquidityPoolShares`.
- `Operation.changeTrust` and `ChangeTrustOp` can now use `LiquidityPoolAsset` in addition to `Asset`.
- `Operation.revokeTrustlineSponsorship` can now use `LiquidityPoolId` in addition to `Asset`.

## [v5.3.2](https://github.com/stellar/js-stellar-base/compare/v5.3.1..v5.3.2)

### Fix
- Update various dependencies to secure versions. Most are developer dependencies which means no or minimal downstream effects ([#446](https://github.com/stellar/js-stellar-base/pull/446), [#447](https://github.com/stellar/js-stellar-base/pull/447), [#392](https://github.com/stellar/js-stellar-base/pull/392), [#428](https://github.com/stellar/js-stellar-base/pull/428)); the only non-developer dependency upgrade is a patch version bump to `lodash` ([#449](https://github.com/stellar/js-stellar-base/pull/449)).


## [v5.3.1](https://github.com/stellar/js-stellar-base/compare/v5.3.0..v5.3.1)

### Fix
- Creating operations with both muxed and unmuxed properties resulted in unintuitive XDR. Specifically, the unmuxed property would be transformed into the equivalent property with an ID of 0 ([#441](https://github.com/stellar/js-stellar-base/pull/441)).


## [v5.3.0](https://github.com/stellar/js-stellar-base/compare/v5.2.1..v5.3.0)

### Add
- **Opt-in support for muxed accounts.** In addition to the support introduced in [v5.2.0](https://github.com/stellar/js-stellar-base/releases/v5.2.0), this completes support for muxed accounts by enabling them for fee-bump transactions. Pass a muxed account address (in the `M...` form) as the first parameter (and explicitly opt-in to muxing by passing `true` as the last parameter) to `TransactionBuilder.buildFeeBumpTransaction` to make the `feeSource` a fully-muxed account instance ([#434](https://github.com/stellar/js-stellar-base/pull/434)).


## [v5.2.1](https://github.com/stellar/js-stellar-base/compare/v5.2.0..v5.2.1)

### Fix
- Fix regression where raw public keys were [sometimes](https://github.com/stellar/js-stellar-sdk/issues/645) being parsed incorrectly ([#429](https://github.com/stellar/js-stellar-base/pull/429)).


## [v5.2.0](https://github.com/stellar/js-stellar-base/compare/v5.1.0..v5.2.0)

### Add
- **Opt-in support for muxed accounts.** This introduces `M...` addresses from [SEP-23](https://stellar.org/protocol/sep-23), which multiplex a Stellar `G...` address across IDs to eliminate the need for ad-hoc multiplexing via the Transaction.memo field (see the relevant [SEP-29](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0029.md) and [blog post](https://www.stellar.org/developers-blog/fixing-memo-less-payments) on the topic). The following operations now support muxed accounts ([#416](https://github.com/stellar/js-stellar-base/pull/416)):
  * `Payment.destination`
  * `PathPaymentStrictReceive.destination`
  * `PathPaymentStrictSend.destination`
  * `Operation.sourceAccount`
  * `AccountMerge.destination`
  * `Transaction.sourceAccount`

- The above changeset also introduces a new high-level object, `MuxedAccount` (not to be confused with `xdr.MuxedAccount`, which is the underlying raw representation) to make working with muxed accounts easier. You can use it to easily create and manage muxed accounts and their underlying shared `Account`, passing them along to the supported operations and `TransactionBuilder` ([#416](https://github.com/stellar/js-stellar-base/pull/416)):

```js
  const PUBKEY = 'GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ';
  const ACC = new StellarBase.Account(PUBKEY, '1');

  const mux1 = new StellarBase.MuxedAccount(ACC, '1000');
  console.log(mux1.accountId(), mux1.id());
  // MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJUAAAAAAAAAAD5DTGC 1000

  const mux2 = ACC.createSubaccount('2000');
  console.log("Parent relationship preserved:",
              mux2.baseAccount().accountId() === mux1.baseAccount().accountId());
  console.log(mux2.accountId(), mux2.id());
  // MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJUAAAAAAAAAAH2B4RU 2000

  mux1.setID('3000');
  console.log("Underlying account unchanged:",
              ACC.accountId() === mux1.baseAccount().accountId());
  console.log(mux1.accountId(), mux1.id());
  // MA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJUAAAAAAAAAALXC5LE 3000
```

- You can refer to the [documentation](https://stellar.github.io/js-stellar-sdk/MuxedAccount.html) or the [test suite](../test/unit/muxed_account_test.js) for more uses of the API.

### Update
- Modernize the minimum-supported browser versions for the library ([#419](https://github.com/stellar/js-stellar-base/pull/419)).

### Fix
- Update Typescript test for `SetOptions` to use authorization flags (e.g. `AuthRequiredFlag`) correctly ([#418](https://github.com/stellar/js-stellar-base/pull/418)).


## [v5.1.0](https://github.com/stellar/js-stellar-base/compare/v5.0.0..v5.1.0)

### Update

- The Typescript definitions have been updated to support CAP-35 ([#407](https://github.com/stellar/js-stellar-base/pull/407)).

## [v5.0.0](https://github.com/stellar/js-stellar-base/compare/v4.0.3..v5.0.0)

### Add

- Introduced new CAP-35 operations, `ClawbackOp`, `ClawbackClaimableBalanceOp`, and `SetTrustLineFlagsOp` ([#397](https://github.com/stellar/js-stellar-base/pull/397/)).

### Update

- Add an additional parameter check to `claimClaimableBalance` to fail faster ([#390](https://github.com/stellar/js-stellar-base/pull/390)).

- The XDR definitions have been updated to support CAP-35 ([#394](https://github.com/stellar/js-stellar-base/pull/394)).

### Breaking

- `AllowTrustOpAsset` has been renamed to `AssetCode` ([#394](https://github.com/stellar/js-stellar-base/pull/394))


### Deprecated

- `AllowTrustOp` is now a deprecated operation.

## [v4.0.3](https://github.com/stellar/js-stellar-base/compare/v4.0.2..v4.0.3)

## Update

- Update TS definitions for XDRs ([#381](https://github.com/stellar/js-stellar-base/pull/381))
- Fix typing for ManageData.value ([#379](https://github.com/stellar/js-stellar-base/pull/379))


## [v4.0.2](https://github.com/stellar/js-stellar-base/compare/v4.0.1..v4.0.2)

## Update

- Fix deployment script.


## [v4.0.1](https://github.com/stellar/js-stellar-base/compare/v4.0.0..v4.0.1)

## Update

- Update `createAccount` operation to accept `0` as the starting balance ([#375](https://github.com/stellar/js-stellar-base/pull/375)).

## [v4.0.0](https://github.com/stellar/js-stellar-base/compare/v3.0.4..v4.0.0)

## Add
- Add the `Claimant` class which helps the creation of claimable balances. ([#367](https://github.com/stellar/js-stellar-base/pull/367)).
The default behavior of this class it to create claimants with an unconditional predicate if none is passed:

```
const claimant = new StellarBase.Claimant(
  'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGSNFHEYVXM3XOJMDS674JZ'
);
```

However, you can use any of the following helpers to create a predicate:

```
StellarBase.Claimant.predicateUnconditional();
StellarBase.Claimant.predicateAnd(left, right);
StellarBase.Claimant.predicateOr(left, right);
StellarBase.Claimant.predicateNot(predicate);
StellarBase.Claimant.predicateBeforeAbsoluteTime(unixEpoch);
StellarBase.Claimant.predicateBeforeRelativeTime(seconds);
```

And then pass the predicate in the constructor:

```
const left = StellarBase.Claimant.predicateBeforeRelativeTime('800');
const right = StellarBase.Claimant.predicateBeforeRelativeTime(
  '1200'
);
const predicate = StellarBase.Claimant.predicateOr(left, right);
const claimant = new StellarBase.Claimant(
  'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGSNFHEYVXM3XOJMDS674JZ',
  predicate
);
```

- Add `Operation.createClaimableBalance` ([#368](https://github.com/stellar/js-stellar-base/pull/368))
Extend the operation class with a new helper to create claimable balance operations.

```js
const asset = new Asset(
  'USD',
  'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7'
);
const amount = '100.0000000';
const claimants = [
  new Claimant(
    'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGSNFHEYVXM3XOJMDS674JZ',
     Claimant.predicateBeforeAbsoluteTime("4102444800000")
  )
];

const op = Operation.createClaimableBalance({
  asset,
  amount,
  claimants
});
```

- Add `Operation.claimClaimableBalance` ([#368](https://github.com/stellar/js-stellar-base/pull/368))
Extend the operation class with a new helper to create claim claimable balance operations. It receives the `balanceId` as exposed by Horizon in the `/claimable_balances` end-point.

```js
const op = Operation.createClaimableBalance({
  balanceId: '00000000da0d57da7d4850e7fc10d2a9d0ebc731f7afb40574c03395b17d49149b91f5be',
});
```
- Add support for Sponsored Reserves (CAP33)([#369](https://github.com/stellar/js-stellar-base/pull/369/))

Extend the operation class with helpers that allow sponsoring reserves and also revoke sponsorships.

To start sponsoring reserves for an account use:
- `Operation.beginSponsoringFutureReserves`
- `Operation.endSponsoringFutureReserves`

To revoke a sponsorship after it has been created use any of the following helpers:

- `Operation.revokeAccountSponsorship`
- `Operation.revokeTrustlineSponsorship`
- `Operation.revokeOfferSponsorship`
- `Operation.revokeDataSponsorship`
- `Operation.revokeClaimableBalanceSponsorship`
- `Operation.revokeSignerSponsorship`

The following example contains a transaction which sponsors operations for an account and then revoke some sponsorships.

```
const transaction = new StellarSdk.TransactionBuilder(account, {
  fee: "100",
  networkPassphrase: StellarSdk.Networks.TESTNET
})
  .addOperation(
    StellarSdk.Operation.beginSponsoringFutureReserves({
      sponsoredId: account.accountId(),
      source: masterKey.publicKey()
    })
  )
  .addOperation(
    StellarSdk.Operation.accountMerge({ destination: destKey.publicKey() }),
  ).addOperation(
    StellarSdk.Operation.createClaimableBalance({
      amount: "10",
      asset: StellarSdk.Asset.native(),
      claimants: [
        new StellarSdk.Claimant(account.accountId())
      ]
    }),
  ).addOperation(
    StellarSdk.Operation.claimClaimableBalance({
      balanceId: "00000000da0d57da7d4850e7fc10d2a9d0ebc731f7afb40574c03395b17d49149b91f5be",
    }),
  ).addOperation(
    StellarSdk.Operation.endSponsoringFutureReserves({
    })
  ).addOperation(
    StellarSdk.Operation.revokeAccountSponsorship({
      account: account.accountId(),
    })
  ).addOperation(
      StellarSdk.Operation.revokeTrustlineSponsorship({
        account: account.accountId(),
        asset: usd,
      })
  ).addOperation(
    StellarSdk.Operation.revokeOfferSponsorship({
      seller: account.accountId(),
      offerId: '12345'
    })
  ).addOperation(
    StellarSdk.Operation.revokeDataSponsorship({
      account: account.accountId(),
      name: 'foo'
    })
  ).addOperation(
    StellarSdk.Operation.revokeClaimableBalanceSponsorship({
      balanceId: "00000000da0d57da7d4850e7fc10d2a9d0ebc731f7afb40574c03395b17d49149b91f5be",
    })
  ).addOperation(
    StellarSdk.Operation.revokeSignerSponsorship({
      account: account.accountId(),
      signer: {
        ed25519PublicKey: sourceKey.publicKey()
      }
    })
  ).addOperation(
    StellarSdk.Operation.revokeSignerSponsorship({
      account: account.accountId(),
      signer: {
        sha256Hash: "da0d57da7d4850e7fc10d2a9d0ebc731f7afb40574c03395b17d49149b91f5be"
      }
    })
  ).addOperation(
    StellarSdk.Operation.revokeSignerSponsorship({
      account: account.accountId(),
      signer: {
        preAuthTx: "da0d57da7d4850e7fc10d2a9d0ebc731f7afb40574c03395b17d49149b91f5be"
      }
    })
  ).build();
```

### Breaking

- The XDR generated in this code includes breaking changes on the internal XDR library since a bug was fixed which was causing incorrect code to be generated (see https://github.com/stellar/xdrgen/pull/52).

The following functions were renamed:

- `xdr.OperationBody.setOption()` -> `xdr.OperationBody.setOptions()`
- `xdr.OperationBody.manageDatum()` -> `xdr.OperationBody.manageData()`
- `xdr.OperationType.setOption()` -> `xdr.OperationType.setOptions()`
- `xdr.OperationType.manageDatum()` -> `xdr.OperationType.manageData()`

The following enum values were rename in `OperationType`:

- `setOption` -> `setOptions`
- `manageDatum` -> `manageData`

## [v3.0.4](https://github.com/stellar/js-stellar-base/compare/v3.0.3..v3.0.4)

### Update

- Generate V1 transactions by default and allow V0 transactions to be fee bumped ([#355](https://github.com/stellar/js-stellar-base/pull/355)).

## [v3.0.3](https://github.com/stellar/js-stellar-base/compare/v3.0.2..v3.0.3)

### Remove

- Rollback support for SEP23 (Muxed Account StrKey) ([#349](https://github.com/stellar/js-stellar-base/pull/349)).

## [v3.0.2](https://github.com/stellar/js-stellar-base/compare/v3.0.1..v3.0.2)

### Fix
- Extend `files` in npm package to include XDR type definitions ([#345](https://github.com/stellar/js-stellar-base/pull/345)).

## [v3.0.1](https://github.com/stellar/js-stellar-base/compare/v3.0.0..v3.0.1)

### Add
- Add TypeScript definitions for auto-generated XDR code ([#342](https://github.com/stellar/js-stellar-base/pull/342)).

## [v3.0.0](https://github.com/stellar/js-stellar-base/compare/v2.1.9..v3.0.0)

This version brings protocol 13 support with backwards compatibility support for protocol 12.

### Add
- Add `TransactionBuilder.buildFeeBumpTransaction` which makes it easy to create `FeeBumpTransaction` ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- Adds a feature flag which allow consumers of this library to create V1 (protocol 13) transactions using the `TransactionBuilder` ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- Add support for [CAP0027](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0027.md): First-class multiplexed accounts ([#325](https://github.com/stellar/js-stellar-base/pull/325)).
- ~Add `Keypair.xdrMuxedAccount` which creates a new `xdr.MuxedAccount`([#325](https://github.com/stellar/js-stellar-base/pull/325)).~
- Add `FeeBumpTransaction` which makes it easy to work with fee bump transactions ([#328](https://github.com/stellar/js-stellar-base/pull/328)).
- Add `TransactionBuilder.fromXDR` which receives an xdr envelope and return a `Transaction` or `FeeBumpTransaction` ([#328](https://github.com/stellar/js-stellar-base/pull/328)).

### Update
- Update XDR definitions with protocol 13 ([#317](https://github.com/stellar/js-stellar-base/pull/317)).
- Extend `Transaction` to work with `TransactionV1Envelope` and `TransactionV0Envelope` ([#317](https://github.com/stellar/js-stellar-base/pull/317)).
- Add backward compatibility support for [CAP0018](https://github.com/stellar/stellar-protocol/blob/f01c9354aaab1e8ca97a25cf888829749cadf36a/core/cap-0018.md) ([#317](https://github.com/stellar/js-stellar-base/pull/317)).
  CAP0018 provides issuers with a new level of authorization between unauthorized and fully authorized, called "authorized to maintain liabilities". The changes in this release allow you to use the new authorization level and provides backward compatible support for Protocol 12.

  Before Protocol 13, the argument `authorize` in the `AllowTrust` operation was of type `boolean` where `true` was authorize and `false` deauthorize. Starting in Protocol 13, this value is now a `number` where `0` is deauthorize, `1` is authorize, and `2` is authorize to maintain liabilities.

  The syntax for authorizing a trustline is still the same, but the authorize parameter is now a `number`.

    ```js
    Operation.allowTrust({
      trustor: trustor.publicKey(),
      assetCode: "COP",
      authorize: 1
    });
    ```

  You can use still use a `boolean`; however, we recommend you update your code to pass a `number` instead. Finally,  using the value `2` for authorize to maintain liabilities will only be valid if Stellar Core is running on Protocol 13; otherwise, you'll get an error.

- ~Update operations builder to support multiplexed accounts ([#337](https://github.com/stellar/js-stellar-base/pull/337)).~

### Breaking changes

- `Transaction.toEnvelope()` returns a protocol 13 `xdr.TransactionEnvelope` which is an `xdr.Union` ([#317](https://github.com/stellar/js-stellar-base/pull/317)).
  If you have code that looks like this - `transaction.toEnvelope().tx` - you have two options:
    - You can grab the value wrapped by the union, calling `value()` like `transaction.toEnvelope().value().tx`.
    - You can check which is the discriminant by using `switch()` and then call `v0()`, `v1()`, or `feeBump()`.
- The return value from `Transaction.fee` changed from `number` to `string`. This brings support for `Int64` values ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- The const `BASE_FEE` changed from `number` to `string` ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- The option `fee` passed to  `new TransactionBuilder({fee: ..})` changed from `number` to `string` ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- The following fields, which were previously an `xdr.AccountID` are now a  `xdr.MuxedAccount` ([#325](https://github.com/stellar/js-stellar-base/pull/325)):
  - `PaymentOp.destination`
  - `PathPaymentStrictReceiveOp.destination`
  - `PathPaymentStrictSendOp.destination`
  - `Operation.sourceAccount`
  - `Operation.destination` (for `ACCOUNT_MERGE`)
  - `Transaction.sourceAccount`
  - `FeeBumpTransaction.feeSource`

  You can get the string representation by calling `StrKey.encodeMuxedAccount` which will return a `G..` or `M..` account.
- Remove the following deprecated functions ([#331](https://github.com/stellar/js-stellar-base/pull/331)):
  - `Operation.manageOffer`
  - `Operation.createPassiveOffer`
  - `Operation.pathPayment`
  - `Keypair.fromBase58Seed`
- Remove the `Network` class ([#331](https://github.com/stellar/js-stellar-base/pull/331)).
- Remove `vendor/base58.js` ([#331](https://github.com/stellar/js-stellar-base/pull/331)).

## [v3.0.0-alpha.1](https://github.com/stellar/js-stellar-base/compare/v3.0.0-alpha.0..v3.0.0-alpha.1)

### Update

- Update operations builder to support multiplexed accounts ([#337](https://github.com/stellar/js-stellar-base/pull/337)).

  This allows you to specify an `M` account as the destination or source:
  ```
  var destination = 'MAAAAAAAAAAAAAB7BQ2L7E5NBWMXDUCMZSIPOBKRDSBYVLMXGSSKF6YNPIB7Y77ITLVL6';
  var amount = '1000.0000000';
  var asset = new StellarBase.Asset(
    'USDUSD',
    'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7'
  );
  var source =
    'MAAAAAAAAAAAAAB7BQ2L7E5NBWMXDUCMZSIPOBKRDSBYVLMXGSSKF6YNPIB7Y77ITLVL6';
  StellarBase.Operation.payment({
    destination,
    asset,
    amount,
    source
  });
  ```

  **To use multiplexed accounts you need an instance of Stellar running on Protocol 13 or higher**

## [v3.0.0-alpha.0](https://github.com/stellar/js-stellar-base/compare/v2.1.9..v3.0.0-alpha.0)

This version brings protocol 13 support with backwards compatibility support for protocol 12.

### Add
- Add `TransactionBuilder.buildFeeBumpTransaction` which makes it easy to create `FeeBumpTransaction` ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- Adds a feature flag which allow consumers of this library to create V1 (protocol 13) transactions using the `TransactionBuilder` ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- Add support for [CAP0027](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0027.md): First-class multiplexed accounts ([#325](https://github.com/stellar/js-stellar-base/pull/325)).
- Add `Keypair.xdrMuxedAccount` which creates a new `xdr.MuxedAccount`([#325](https://github.com/stellar/js-stellar-base/pull/325)).
- Add `FeeBumpTransaction` which makes it easy to work with fee bump transactions ([#328](https://github.com/stellar/js-stellar-base/pull/328)).
- Add `TransactionBuilder.fromXDR` which receives an xdr envelope and return a `Transaction` or `FeeBumpTransaction` ([#328](https://github.com/stellar/js-stellar-base/pull/328)).

### Update
- Update XDR definitions with protocol 13 ([#317](https://github.com/stellar/js-stellar-base/pull/317)).
- Extend `Transaction` to work with `TransactionV1Envelope` and `TransactionV0Envelope` ([#317](https://github.com/stellar/js-stellar-base/pull/317)).
- Add backward compatibility support for [CAP0018](https://github.com/stellar/stellar-protocol/blob/f01c9354aaab1e8ca97a25cf888829749cadf36a/core/cap-0018.md) ([#317](https://github.com/stellar/js-stellar-base/pull/317)).

### Breaking changes

- `Transaction.toEnvelope()` returns a protocol 13 `xdr.TransactionEnvelope` which is an `xdr.Union` ([#317](https://github.com/stellar/js-stellar-base/pull/317)).
  If you have code that looks like this `transaction.toEnvelope().tx` you have two options:
    - You can grab the value wrapped by the union, calling `value()` like `transaction.toEnvelope().value().tx`.
    - You can check which is the discriminant by using `switch()` and then call `v0()`, `v1()`, or `feeBump()`.
- The return value from `Transaction.fee` changed from `number` to `string`. This brings support for `Int64` values ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- The const `BASE_FEE` changed from `number` to `string` ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- The option `fee` passed to  `new TransactionBuilder({fee: ..})` changed from `number` to `string` ([#321](https://github.com/stellar/js-stellar-base/pull/321)).
- The following fields, which were previously an `xdr.AccountID` are now a  `xdr.MuxedAccount` ([#325](https://github.com/stellar/js-stellar-base/pull/325)):
  - `PaymentOp.destination`
  - `PathPaymentStrictReceiveOp.destination`
  - `PathPaymentStrictSendOp.destination`
  - `Operation.sourceAccount`
  - `Operation.destination` (for `ACCOUNT_MERGE`)
  - `Transaction.sourceAccount`
  - `FeeBumpTransaction.feeSource`

  You can get the string representation by calling `StrKey.encodeMuxedAccount` which will return a `G..` or `M..` account.
- Remove the following deprecated functions ([#331](https://github.com/stellar/js-stellar-base/pull/331)):
  - `Operation.manageOffer`
  - `Operation.createPassiveOffer`
  - `Operation.pathPayment`
  - `Keypair.fromBase58Seed`
- Remove the `Network` class ([#331](https://github.com/stellar/js-stellar-base/pull/331)).
- Remove `vendor/base58.js` ([#331](https://github.com/stellar/js-stellar-base/pull/331)).


## [v2.1.9](https://github.com/stellar/js-stellar-base/compare/v2.1.8..v2.1.9)

### Fix
- Update dependencies which depend on minimist. ([#332](https://github.com/stellar/js-stellar-base/pull/332))

## [v2.1.8](https://github.com/stellar/js-stellar-base/compare/v2.1.7..v2.1.8)

### Fix
- Fix `setTimeout(0)` and partially defined timebounds ([#315](https://github.com/stellar/js-stellar-base/pull/315)).

## [v2.1.7](https://github.com/stellar/js-stellar-base/compare/v2.1.6..v2.1.7)

### Fix
- Fix TypeScript options for `ManageData` operation to allow setting value to `null` ([#310](https://github.com/stellar/js-stellar-base/issues/310))
- Fix crash on partially defined time bounds ([#303](https://github.com/stellar/js-stellar-base/issues/303))

## [v2.1.6](https://github.com/stellar/js-stellar-base/compare/v2.1.5..v2.1.6)

### Fix
- Fix npm deployment.

## [v2.1.5](https://github.com/stellar/js-stellar-base/compare/v2.1.4..v2.1.5)

### Add
- Add `toXDR` type to Transaction class ([#296](https://github.com/stellar/js-stellar-base/issues/296))

### Fix
- Fix doc link ([#298](https://github.com/stellar/js-stellar-base/issues/298))

### Remove
- Remove node engine restriction ([#294](https://github.com/stellar/js-stellar-base/issues/294))

### Update
- Update creating an account example ([#299](https://github.com/stellar/js-stellar-base/issues/299))
- Use `console.trace` to get line num in `Networks.use` ([#300](https://github.com/stellar/js-stellar-base/issues/300))

## [v2.1.4](https://github.com/stellar/js-stellar-base/compare/v2.1.3..v2.1.4)

## Update
- Regenerate the XDR definitions to include MetaV2 ([#288](https://github.com/stellar/js-stellar-base/issues/288))

## [v2.1.3](https://github.com/stellar/js-stellar-base/compare/v2.1.2...v2.1.3)

## Update 📣

- Throw errors when obviously invalid network passphrases are used in
  `new Transaction()`.
  ([284](https://github.com/stellar/js-stellar-base/pull/284))

## [v2.1.2](https://github.com/stellar/js-stellar-base/compare/v2.1.1...v2.1.2)

## Update 📣

- Update documentation for `Operation` to show `pathPaymentStrictSend` and `pathPaymentStrictReceive`. ([279](https://github.com/stellar/js-stellar-base/pull/279))

## [v2.1.1](https://github.com/stellar/js-stellar-base/compare/v2.1.0...v2.1.1)

## Update 📣

- Update `asset.toString()` to return canonical representation for asset. ([277](https://github.com/stellar/js-stellar-base/pull/277)).

  Calling `asset.toString()` will return `native` for `XLM` or `AssetCode:AssetIssuer` for issued assets. See [this PR](https://github.com/stellar/stellar-protocol/pull/313) for more information.

## [v2.1.0](https://github.com/stellar/js-stellar-base/compare/v2.0.2...v2.1.0)

This release adds support for [stellar-core protocol 12 release](https://github.com/stellar/stellar-core/projects/11) and [CAP 24](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0024.md) ("Make PathPayment Symmetrical").

### Add ➕

 - `Operation.pathPaymentStrictSend`: Sends a path payments, debiting from the source account exactly a specified amount of one asset, crediting at least a given amount of another asset. ([#274](https://github.com/stellar/js-stellar-base/pull/274)).

    The following operation will debit exactly 10 USD from the source account, crediting at least 9.2 EUR in the destination account 💸:
    ```js
    var sendAsset = new StellarBase.Asset(
      'USD',
      'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7'
    );
    var sendAmount = '10';
    var destination =
      'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGSNFHEYVXM3XOJMDS674JZ';
    var destAsset = new StellarBase.Asset(
      'USD',
      'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7'
    );
    var destMin = '9.2';
    var path = [
      new StellarBase.Asset(
        'USD',
        'GBBM6BKZPEHWYO3E3YKREDPQXMS4VK35YLNU7NFBRI26RAN7GI5POFBB'
      ),
      new StellarBase.Asset(
        'EUR',
        'GDTNXRLOJD2YEBPKK7KCMR7J33AAG5VZXHAJTHIG736D6LVEFLLLKPDL'
      )
    ];
    let op = StellarBase.Operation.pathPaymentStrictSend({
      sendAsset,
      sendAmount,
      destination,
      destAsset,
      destMin,
      path
    });
    ```
 - `Operation.pathPaymentStrictReceive`: This behaves the same as the former `pathPayments` operation. ([#274](https://github.com/stellar/js-stellar-base/pull/274)).

   The following operation will debit maximum 10 USD from the source account, crediting exactly 9.2 EUR in the destination account  💸:
   ```js
   var sendAsset = new StellarBase.Asset(
     'USD',
     'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7'
   );
   var sendMax = '10';
   var destination =
     'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGSNFHEYVXM3XOJMDS674JZ';
   var destAsset = new StellarBase.Asset(
     'USD',
     'GDGU5OAPHNPU5UCLE5RDJHG7PXZFQYWKCFOEXSXNMR6KRQRI5T6XXCD7'
   );
   var destAmount = '9.2';
   var path = [
     new StellarBase.Asset(
       'USD',
       'GBBM6BKZPEHWYO3E3YKREDPQXMS4VK35YLNU7NFBRI26RAN7GI5POFBB'
     ),
     new StellarBase.Asset(
       'EUR',
       'GDTNXRLOJD2YEBPKK7KCMR7J33AAG5VZXHAJTHIG736D6LVEFLLLKPDL'
     )
   ];
   let op = StellarBase.Operation.pathPaymentStrictReceive({
     sendAsset,
     sendMax,
     destination,
     destAsset,
     destAmount,
     path
   });
   ```

## Deprecated ❗️

- `Operation.pathPayment` is being deprecated in favor of `Operation.pathPaymentStrictReceive`. Both functions take the same arguments and behave the same. ([#274](https://github.com/stellar/js-stellar-base/pull/274)).

## [v2.0.2](https://github.com/stellar/js-stellar-base/compare/v2.0.1...v2.0.2)

### Fix
- Fix issue [#269](https://github.com/stellar/js-stellar-base/issues/269). ManageBuyOffer should extend BaseOptions and inherited property "source". ([#270](https://github.com/stellar/js-stellar-base/pull/270)).

## [v2.0.1](https://github.com/stellar/js-stellar-base/compare/v2.0.0...v2.0.1)

No changes. Fixes deploy script and includes changes from [v2.0.0](https://github.com/stellar/js-stellar-base/compare/v1.1.2...v2.0.0).

## [v2.0.0](https://github.com/stellar/js-stellar-base/compare/v1.1.2...v2.0.0)

### BREAKING CHANGES

- Drop Support for Node 6 since it has been end-of-lifed and no longer in LTS. We now require Node 10 which is the current LTS until April 1st, 2021. ([#255](https://github.com/stellar/js-stellar-base/pull/255))

## [v1.1.2](https://github.com/stellar/js-stellar-base/compare/v1.1.1...v1.1.2)

### Fix
- Fix no-network warnings ([#248](https://github.com/stellar/js-stellar-base/issues/248))

## [v1.1.1](https://github.com/stellar/js-stellar-base/compare/v1.1.0...v1.1.1)

### Fix
- Add types for new networkPassphrase argument. Fix [#237](https://github.com/stellar/js-stellar-base/issues/237). ([#238](https://github.com/stellar/js-stellar-base/issues/238))

## [v1.1.0](https://github.com/stellar/js-stellar-base/compare/v1.0.3...v1.1.0)

### Deprecated

Deprecate global singleton for `Network`. The following classes and
methods take an optional network passphrase, and issue a warning if it
is not passed:

#### `Keypair.master`

```js
Keypair.master(Networks.TESTNET)
```

#### constructor for `Transaction`

```js
const xenv = new xdr.TransactionEnvelope({ tx: xtx });
new Transaction(xenv, Networks.TESTNET);
```

#### constructor for  `TransactionBuilder` and method `TransactionBuilder.setNetworkPassphrase`

```js
const transaction = new StellarSdk.TransactionBuilder(account, {
  fee: StellarSdk.BASE_FEE,
  networkPassphrase: Networks.TESTNET
})
```

See [#207](https://github.com/stellar/js-stellar-base/issues/207) and [#112](https://github.com/stellar/js-stellar-base/issues/112) for more information.

The `Network` class will be removed on the `2.0` release.

### Add
- Add docs for BASE_FEE const. ([#211](https://github.com/stellar/js-stellar-base/issues/211))

### Fix
- Fix typo. ([#213](https://github.com/stellar/js-stellar-base/issues/213))

## [v1.0.3](https://github.com/stellar/js-stellar-base/compare/v1.0.2...v1.0.3)

### Add

- Add `toString()` to Asset ([#172](https://github.com/stellar/js-stellar-base/issues/172))
- Add types for missing Network functions ([#208](https://github.com/stellar/js-stellar-base/issues/208))
- Add BASE_FEE to TS types ([#209](https://github.com/stellar/js-stellar-base/issues/209))

### Fix
- Fix typo in types ([#194](https://github.com/stellar/js-stellar-base/issues/194))
- Fix types: Fee is no longer optional ([#195](https://github.com/stellar/js-stellar-base/issues/195))
- Fix typings for Account Sequence Number ([#203](https://github.com/stellar/js-stellar-base/issues/203))
- Fix typings for Transaction Sequence Number ([#205](https://github.com/stellar/js-stellar-base/issues/205))

## [v1.0.2](https://github.com/stellar/js-stellar-base/compare/v1.0.1...v1.0.2)

- Fix a bug where `sodium-native` was making it into the browser bundle, which
  is supposed to use `tweetnacl`.

## [v1.0.1](https://github.com/stellar/js-stellar-base/compare/v1.0.0...v1.0.1)

- Restore `Operation.manageOffer` and `Operation.createPassiveOffer`, and issue
  a warning if they're called.
- Add type definitions for the timeBounds property of transactions.

## [v1.0.0](https://github.com/stellar/js-stellar-base/compare/v0.13.2...v1.0.0)

- **Breaking change** Stellar Protocol 11 compatibility
  - Rename `Operation.manageOffer` to `Operation.manageSellOffer`.
  - Rename `Operation.createPassiveOffer` to `Operation.createPassiveSellOffer`.
  - Add `Operation.manageBuyOffer`.
- **Breaking change** The `fee` parameter to `TransactionBuilder` is now
  required. Failing to provide a fee will throw an error.

## [v0.13.2](https://github.com/stellar/js-stellar-base/compare/v0.13.1...v0.13.2)

- Bring DefinitelyTyped definitions into the repo for faster updating.
- Add missing Typescript type definitions.
- Add code to verify signatures when added to transactions.
- Replace ed25519 with sodium-native.
- Fix the xdr for SCP_MESSAGE.
- Update the README for the latest info.

## [v0.13.1](https://github.com/stellar/js-stellar-base/compare/v0.13.0...v0.13.1)

- Travis: Deploy NPM with an environment variable instead of an encrypted API
  key.
- Instruct Travis to cache node_modules

## [v0.13.0](https://github.com/stellar/js-stellar-base/compare/v0.12.0...v0.13.0)

- Remove the `crypto` library. This reduces the number of Node built-ins we have
  to shim into the production bundle, and incidentally fixes a bug with
  Angular 6.

## [v0.12.0](https://github.com/stellar/js-stellar-base/compare/v0.11.0...v0.12.0)

- _Warning_ Calling TransactionBuilder without a `fee` param is now deprecated
  and will issue a warning. In a later release, it will throw an error. Please
  update your transaction builders as soon as you can!
- Add a `toXDR` function for transactions that lets you get the transaction as a
  base64-encoded string (so you may enter it into the Stellar Laboratory XDR
  viewer, for one)
- Fix TransactionBuilder example syntax errors
- Use more thorough "create account" documentation
- Add `Date` support for `TransactionBuilder` `timebounds`
- Add two functions to `Transaction` that support pre-generated transactions:
  - `getKeypairSignature` helps users sign pre-generated transaction XDRs
  - `addSignature` lets you add pre-generated signatures to a built transaction

## 0.11.0

- Added ESLint and Prettier to enforce code style
- Upgraded dependencies, including Babel to 6
- Bump local node version to 6.14.0
- Change Operations.\_fromXDRAmount to not use scientific notation (1e-7) for
  small amounts like 0.0000001.

## 0.10.0

- **Breaking change** Added
  [`TransactionBuilder.setTimeout`](https://stellar.github.io/js-stellar-base/TransactionBuilder.html#setTimeout)
  method that sets `timebounds.max_time` on a transaction. Because of the
  distributed nature of the Stellar network it is possible that the status of
  your transaction will be determined after a long time if the network is highly
  congested. If you want to be sure to receive the status of the transaction
  within a given period you should set the TimeBounds with `maxTime` on the
  transaction (this is what `setTimeout` does internally; if there's `minTime`
  set but no `maxTime` it will be added). Call to
  `TransactionBuilder.setTimeout` is required if Transaction does not have
  `max_time` set. If you don't want to set timeout, use `TimeoutInfinite`. In
  general you should set `TimeoutInfinite` only in smart contracts. Please check
  [`TransactionBuilder.setTimeout`](https://stellar.github.io/js-stellar-base/TransactionBuilder.html#setTimeout)
  docs for more information.
- Fixed decoding empty `homeDomain`.

## 0.9.0

- Update `js-xdr` to support unmarshaling non-utf8 strings.
- String fields returned by `Operation.fromXDRObject()` are of type `Buffer` now
  (except `SetOptions.home_domain` and `ManageData.name` - both required to be
  ASCII by stellar-core).

## 0.8.3

- Update `xdr` files to V10.

## 0.8.2

- Upgrade `js-xdr`.

## 0.8.1

- Removed `src` from `.npmignore`.

## 0.8.0

- Added support for `bump_sequence` operation.
- Fixed many code style issues.
- Updated docs.

## 0.7.8

- Updated dependencies.

## 0.7.7

- Updated docs.

## 0.7.6

- Updated docs.

## 0.7.5

- `Keypair.constructor` now requires `type` field to define public-key signature
  system used in this instance (so `Keypair` can support other systems in a
  future). It also checks if public key and secret key match if both are passed
  (to prevent nasty bugs).
- `Keypair.fromRawSeed` has been renamed to `Keypair.fromRawEd25519Seed` to make
  it clear that the seed must be Ed25519 seed.
- It's now possible to instantiate `Memo` class so it's easier to check it's
  type and value (without dealing with low level `xdr.Memo` objects).
- Changed `Asset.toXdrObject` to `Asset.toXDRObject` and
  `Operation.operationToObject` to `Operation.toXDRObject` for consistency.
- Time bounds support for numeric input values.
- Added `browser` prop to package.json.

## 0.7.4

- Update dependencies.
- Remove unused methods.

## 0.7.3

- Allow hex string in setOptions signers

## 0.7.2

- Updated XDR files

## 0.7.1

- Checking hash preimage length

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

## 0.6.0

- **Breaking change** `ed25519` package is now optional dependency.
- Export account flags constants.

## 0.5.7

- Fixes XDR decoding issue when using firefox

## 0.5.6

- UTF-8 support in `Memo.text()`.

## 0.5.5

- Make 0 a valid number for transaction fee,
- Fix signer in Operation.operationToObject() - close #82

## 0.5.4

- Fixed Lodash registering itself to global scope.

## 0.5.3

- Add support for ManageData operation.

## 0.5.2

- Moved `Account.isValidAccountId` to `Keypair.isValidPublicKey`. It's still
  possible to use `Account.isValidAccountId` but it will be removed in the next
  minor release (breaking change). (af10f2a)
- `signer.address` option in `Operation.setOptions` was changed to
  `signer.pubKey`. It's still possible to use `signer.address` but it will be
  removed in the next minor release (breaking change). (07f43fb)
- `Operation.setOptions` now accepts strings for `clearFlags`, `setFlags`,
  `masterWeight`, `lowThreshold`, `medThreshold`, `highThreshold`,
  `signer.weight` options. (665e018)
- Fixed TransactionBuilder timebounds option. (854f275)
- Added `CHANGELOG.md` file.

## 0.5.1

- Now it's possible to pass `price` params as `{n: numerator, d: denominator}`
  object. Thanks @FredericHeem. (#73)

## 0.5.0

- **Breaking change** `sequence` in `Account` constructor must be a string.
  (4da5dfc)
- **Breaking change** Removed deprecated methods (180a5b8):
  - `Account.isValidAddress` (replaced by `Account.isValidAccountId`)
  - `Account.getSequenceNumber` (replaced by `Account.sequenceNumber`)
  - `Keypair.address` (replaced by `Keypair.accountId`)
  - `Network.usePublicNet` (replaced by `Network.usePublicNetwork`)
  - `Network.useTestNet` (replaced by `Network.useTestNetwork`)
  - `TransactionBuilder.addSigner` (call `Transaction.sign` on build
    `Transaction` object)
