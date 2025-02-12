"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.createCustomContract = createCustomContract;
exports.createStellarAssetContract = createStellarAssetContract;
exports.invokeContractFunction = invokeContractFunction;
exports.invokeHostFunction = invokeHostFunction;
exports.uploadContractWasm = uploadContractWasm;
var _xdr = _interopRequireDefault(require("../xdr"));
var _keypair = require("../keypair");
var _address = require("../address");
var _asset = require("../asset");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { "default": e }; }
function _slicedToArray(r, e) { return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest(); }
function _nonIterableRest() { throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method."); }
function _unsupportedIterableToArray(r, a) { if (r) { if ("string" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _iterableToArrayLimit(r, l) { var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"]; if (null != t) { var e, n, i, u, a = [], f = !0, o = !1; try { if (i = (t = t.call(r)).next, 0 === l) { if (Object(t) !== t) return; f = !1; } else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l); f = !0); } catch (r) { o = !0, n = r; } finally { try { if (!f && null != t["return"] && (u = t["return"](), Object(u) !== u)) return; } finally { if (o) throw n; } } return a; } }
function _arrayWithHoles(r) { if (Array.isArray(r)) return r; }
/**
 * Invokes a single smart contract host function.
 *
 * @function
 * @alias Operation.invokeHostFunction
 *
 * @param {object} opts - options object
 * @param {xdr.HostFunction} opts.func - host function to execute (with its
 *    wrapped parameters)
 * @param {xdr.SorobanAuthorizationEntry[]} [opts.auth] - list outlining the
 *    tree of authorizations required for the call
 * @param {string} [opts.source] - an optional source account
 *
 * @returns {xdr.Operation} an Invoke Host Function operation
 *    (xdr.InvokeHostFunctionOp)
 *
 * @see https://soroban.stellar.org/docs/fundamentals-and-concepts/invoking-contracts-with-transactions#function
 * @see Operation.invokeContractFunction
 * @see Operation.createCustomContract
 * @see Operation.createStellarAssetContract
 * @see Operation.uploadContractWasm
 * @see Contract.call
 */
function invokeHostFunction(opts) {
  if (!opts.func) {
    throw new TypeError("host function invocation ('func') required (got ".concat(JSON.stringify(opts), ")"));
  }
  var invokeHostFunctionOp = new _xdr["default"].InvokeHostFunctionOp({
    hostFunction: opts.func,
    auth: opts.auth || []
  });
  var opAttributes = {
    body: _xdr["default"].OperationBody.invokeHostFunction(invokeHostFunctionOp)
  };
  this.setSourceAccount(opAttributes, opts);
  return new _xdr["default"].Operation(opAttributes);
}

/**
 * Returns an operation that invokes a contract function.
 *
 * @function
 * @alias Operation.invokeContractFunction
 *
 * @param {any}         opts - the set of parameters
 * @param {string}      opts.contract - a strkey-fied contract address (`C...`)
 * @param {string}      opts.function - the name of the contract fn to invoke
 * @param {xdr.ScVal[]} opts.args - parameters to pass to the function
 *    invocation (try {@link nativeToScVal} or {@link ScInt} to make building
 *    these easier)
 * @param {xdr.SorobanAuthorizationEntry[]} [opts.auth] - an optional list
 *    outlining the tree of authorizations required for the call
 * @param {string} [opts.source] - an optional source account
 *
 * @returns {xdr.Operation} an Invoke Host Function operation
 *    (xdr.InvokeHostFunctionOp)
 *
 * @see Operation.invokeHostFunction
 * @see Contract.call
 * @see Address
 */
function invokeContractFunction(opts) {
  var c = new _address.Address(opts.contract);
  if (c._type !== 'contract') {
    throw new TypeError("expected contract strkey instance, got ".concat(c));
  }
  return this.invokeHostFunction({
    source: opts.source,
    auth: opts.auth,
    func: _xdr["default"].HostFunction.hostFunctionTypeInvokeContract(new _xdr["default"].InvokeContractArgs({
      contractAddress: c.toScAddress(),
      functionName: opts["function"],
      args: opts.args
    }))
  });
}

/**
 * Returns an operation that creates a custom WASM contract and atomically
 * invokes its constructor.
 *
 * @function
 * @alias Operation.createCustomContract
 *
 * @param {any}     opts - the set of parameters
 * @param {Address} opts.address - the contract uploader address
 * @param {Uint8Array|Buffer}  opts.wasmHash - the SHA-256 hash of the contract
 *    WASM you're uploading (see {@link hash} and
 *    {@link Operation.uploadContractWasm})
 * @param {xdr.ScVal[]} [opts.constructorArgs] - the optional parameters to pass
 *    to the constructor of this contract (see {@link nativeToScVal} for ways to
 *    easily create these parameters from native JS values)
 * @param {Uint8Array|Buffer} [opts.salt] - an optional, 32-byte salt to
 *    distinguish deployment instances of the same wasm from the same user (if
 *    omitted, one will be generated for you)
 * @param {xdr.SorobanAuthorizationEntry[]} [opts.auth] - an optional list
 *    outlining the tree of authorizations required for the call
 * @param {string} [opts.source] - an optional source account
 *
 * @returns {xdr.Operation} an Invoke Host Function operation
 *    (xdr.InvokeHostFunctionOp)
 *
 * @see
 * https://soroban.stellar.org/docs/fundamentals-and-concepts/invoking-contracts-with-transactions#function
 */
function createCustomContract(opts) {
  var _opts$constructorArgs;
  var salt = Buffer.from(opts.salt || getSalty());
  if (!opts.wasmHash || opts.wasmHash.length !== 32) {
    throw new TypeError("expected hash(contract WASM) in 'opts.wasmHash', got ".concat(opts.wasmHash));
  }
  if (salt.length !== 32) {
    throw new TypeError("expected 32-byte salt in 'opts.salt', got ".concat(opts.wasmHash));
  }
  return this.invokeHostFunction({
    source: opts.source,
    auth: opts.auth,
    func: _xdr["default"].HostFunction.hostFunctionTypeCreateContractV2(new _xdr["default"].CreateContractArgsV2({
      executable: _xdr["default"].ContractExecutable.contractExecutableWasm(Buffer.from(opts.wasmHash)),
      contractIdPreimage: _xdr["default"].ContractIdPreimage.contractIdPreimageFromAddress(new _xdr["default"].ContractIdPreimageFromAddress({
        address: opts.address.toScAddress(),
        salt: salt
      })),
      constructorArgs: (_opts$constructorArgs = opts.constructorArgs) !== null && _opts$constructorArgs !== void 0 ? _opts$constructorArgs : []
    }))
  });
}

/**
 * Returns an operation that wraps a Stellar asset into a token contract.
 *
 * @function
 * @alias Operation.createStellarAssetContract
 *
 * @param {any}          opts - the set of parameters
 * @param {Asset|string} opts.asset - the Stellar asset to wrap, either as an
 *    {@link Asset} object or in canonical form (SEP-11, `code:issuer`)
 * @param {xdr.SorobanAuthorizationEntry[]} [opts.auth] - an optional list
 *    outlining the tree of authorizations required for the call
 * @param {string} [opts.source] - an optional source account
 *
 * @returns {xdr.Operation} an Invoke Host Function operation
 *    (xdr.InvokeHostFunctionOp)
 *
 * @see https://stellar.org/protocol/sep-11#alphanum4-alphanum12
 * @see
 * https://soroban.stellar.org/docs/fundamentals-and-concepts/invoking-contracts-with-transactions
 * @see
 * https://soroban.stellar.org/docs/advanced-tutorials/stellar-asset-contract
 * @see Operation.invokeHostFunction
 */
function createStellarAssetContract(opts) {
  var asset = opts.asset;
  if (typeof asset === 'string') {
    var _asset$split = asset.split(':'),
      _asset$split2 = _slicedToArray(_asset$split, 2),
      code = _asset$split2[0],
      issuer = _asset$split2[1];
    asset = new _asset.Asset(code, issuer); // handles 'xlm' by default
  }
  if (!(asset instanceof _asset.Asset)) {
    throw new TypeError("expected Asset in 'opts.asset', got ".concat(asset));
  }
  return this.invokeHostFunction({
    source: opts.source,
    auth: opts.auth,
    func: _xdr["default"].HostFunction.hostFunctionTypeCreateContract(new _xdr["default"].CreateContractArgs({
      executable: _xdr["default"].ContractExecutable.contractExecutableStellarAsset(),
      contractIdPreimage: _xdr["default"].ContractIdPreimage.contractIdPreimageFromAsset(asset.toXDRObject())
    }))
  });
}

/**
 * Returns an operation that uploads WASM for a contract.
 *
 * @function
 * @alias Operation.uploadContractWasm
 *
 * @param {any}               opts - the set of parameters
 * @param {Uint8Array|Buffer} opts.wasm - a WASM blob to upload to the ledger
 * @param {xdr.SorobanAuthorizationEntry[]} [opts.auth] - an optional list
 *    outlining the tree of authorizations required for the call
 * @param {string} [opts.source] - an optional source account
 *
 * @returns {xdr.Operation} an Invoke Host Function operation
 *    (xdr.InvokeHostFunctionOp)
 *
 * @see
 * https://soroban.stellar.org/docs/fundamentals-and-concepts/invoking-contracts-with-transactions#function
 */
function uploadContractWasm(opts) {
  return this.invokeHostFunction({
    source: opts.source,
    auth: opts.auth,
    func: _xdr["default"].HostFunction.hostFunctionTypeUploadContractWasm(Buffer.from(opts.wasm) // coalesce so we can drop `Buffer` someday
    )
  });
}

/** @returns {Buffer} a random 256-bit "salt" value. */
function getSalty() {
  return _keypair.Keypair.random().xdrPublicKey().value(); // ed25519 is 256 bits, too
}