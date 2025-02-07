"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.buildInvocationTree = buildInvocationTree;
exports.walkInvocationTree = walkInvocationTree;
var _asset = require("./asset");
var _address = require("./address");
var _scval = require("./scval");
function _typeof(o) { "@babel/helpers - typeof"; return _typeof = "function" == typeof Symbol && "symbol" == typeof Symbol.iterator ? function (o) { return typeof o; } : function (o) { return o && "function" == typeof Symbol && o.constructor === Symbol && o !== Symbol.prototype ? "symbol" : typeof o; }, _typeof(o); }
function ownKeys(e, r) { var t = Object.keys(e); if (Object.getOwnPropertySymbols) { var o = Object.getOwnPropertySymbols(e); r && (o = o.filter(function (r) { return Object.getOwnPropertyDescriptor(e, r).enumerable; })), t.push.apply(t, o); } return t; }
function _objectSpread(e) { for (var r = 1; r < arguments.length; r++) { var t = null != arguments[r] ? arguments[r] : {}; r % 2 ? ownKeys(Object(t), !0).forEach(function (r) { _defineProperty(e, r, t[r]); }) : Object.getOwnPropertyDescriptors ? Object.defineProperties(e, Object.getOwnPropertyDescriptors(t)) : ownKeys(Object(t)).forEach(function (r) { Object.defineProperty(e, r, Object.getOwnPropertyDescriptor(t, r)); }); } return e; }
function _defineProperty(e, r, t) { return (r = _toPropertyKey(r)) in e ? Object.defineProperty(e, r, { value: t, enumerable: !0, configurable: !0, writable: !0 }) : e[r] = t, e; }
function _toPropertyKey(t) { var i = _toPrimitive(t, "string"); return "symbol" == _typeof(i) ? i : i + ""; }
function _toPrimitive(t, r) { if ("object" != _typeof(t) || !t) return t; var e = t[Symbol.toPrimitive]; if (void 0 !== e) { var i = e.call(t, r || "default"); if ("object" != _typeof(i)) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === r ? String : Number)(t); }
/**
 * @typedef CreateInvocation
 *
 * @prop {'wasm'|'sac'} type  a type indicating if this creation was a custom
 *    contract or a wrapping of an existing Stellar asset
 * @prop {string} [token] when `type=='sac'`, the canonical {@link Asset} that
 *    is being wrapped by this Stellar Asset Contract
 * @prop {object} [wasm]  when `type=='wasm'`, add'l creation parameters
 *
 * @prop {string} wasm.hash     hex hash of WASM bytecode backing this contract
 * @prop {string} wasm.address  contract address of this deployment
 * @prop {string} wasm.salt     hex salt that the user consumed when creating
 *    this contract (encoded in the resulting address)
 * @prop {any[]}  [wasm.constructorArgs] a list of natively-represented values
 *    (see {@link scValToNative}) that are passed to the constructor when
 *    creating this contract
 */

/**
 * @typedef ExecuteInvocation
 *
 * @prop {string} source    the strkey of the contract (C...) being invoked
 * @prop {string} function  the name of the function being invoked
 * @prop {any[]}  args      the natively-represented parameters to the function
 *    invocation (see {@link scValToNative} for rules on how they're
 *    represented a JS types)
 */

/**
 * @typedef InvocationTree
 * @prop {'execute' | 'create'} type  the type of invocation occurring, either
 *    contract creation or host function execution
 * @prop {CreateInvocation | ExecuteInvocation} args  the parameters to the
 *    invocation, depending on the type
 * @prop {InvocationTree[]} invocations   any sub-invocations that (may) occur
 *    as a result of this invocation (i.e. a tree of call stacks)
 */

/**
 * Turns a raw invocation tree into a human-readable format.
 *
 * This is designed to make the invocation tree easier to understand in order to
 * inform users about the side-effects of their contract calls. This will help
 * make informed decisions about whether or not a particular invocation will
 * result in what you expect it to.
 *
 * @param {xdr.SorobanAuthorizedInvocation} root  the raw XDR of the invocation,
 *    likely acquired from transaction simulation. this is either from the
 *    {@link Operation.invokeHostFunction} itself (the `func` field), or from
 *    the authorization entries ({@link xdr.SorobanAuthorizationEntry}, the
 *    `rootInvocation` field)
 *
 * @returns {InvocationTree}  a human-readable version of the invocation tree
 *
 * @example
 * Here, we show a browser modal after simulating an arbitrary transaction,
 * `tx`, which we assume has an `Operation.invokeHostFunction` inside of it:
 *
 * ```typescript
 * import { Server, buildInvocationTree } from '@stellar/stellar-sdk';
 *
 * const s = new Server("fill in accordingly");
 *
 * s.simulateTransaction(tx).then(
 *  (resp: SorobanRpc.SimulateTransactionResponse) => {
 *    if (SorobanRpc.isSuccessfulSim(resp) && ) {
 *      // bold assumption: there's a valid result with an auth entry
 *      alert(
 *        "You are authorizing the following invocation:\n" +
 *        JSON.stringify(
 *          buildInvocationTree(resp.result!.auth[0].rootInvocation()),
 *          null,
 *          2
 *        )
 *      );
 *    }
 *  }
 * );
 * ```
 */
function buildInvocationTree(root) {
  var fn = root["function"]();

  /** @type {InvocationTree} */
  var output = {};

  /** @type {xdr.CreateContractArgs|xdr.CreateContractArgsV2|xdr.InvokeContractArgs} */
  var inner = fn.value();
  switch (fn["switch"]().value) {
    // sorobanAuthorizedFunctionTypeContractFn
    case 0:
      output.type = 'execute';
      output.args = {
        source: _address.Address.fromScAddress(inner.contractAddress()).toString(),
        "function": inner.functionName(),
        args: inner.args().map(function (arg) {
          return (0, _scval.scValToNative)(arg);
        })
      };
      break;

    // sorobanAuthorizedFunctionTypeCreateContractHostFn
    // sorobanAuthorizedFunctionTypeCreateContractV2HostFn
    case 1: // fallthrough: just no ctor args in V1
    case 2:
      {
        var createV2 = fn["switch"]().value === 2;
        output.type = 'create';
        output.args = {};

        // If the executable is a WASM, the preimage MUST be an address. If it's a
        // token, the preimage MUST be an asset. This is a cheeky way to check
        // that, because wasm=0, token=1 and address=0, asset=1 in the XDR switch
        // values.
        //
        // The first part may not be true in V2, but we'd need to update this code
        // anyway so it can still be an error.
        var _ref = [inner.executable(), inner.contractIdPreimage()],
          exec = _ref[0],
          preimage = _ref[1];
        if (!!exec["switch"]().value !== !!preimage["switch"]().value) {
          throw new Error("creation function appears invalid: ".concat(JSON.stringify(inner), " (should be wasm+address or token+asset)"));
        }
        switch (exec["switch"]().value) {
          // contractExecutableWasm
          case 0:
            {
              /** @type {xdr.ContractIdPreimageFromAddress} */
              var details = preimage.fromAddress();
              output.args.type = 'wasm';
              output.args.wasm = _objectSpread({
                salt: details.salt().toString('hex'),
                hash: exec.wasmHash().toString('hex'),
                address: _address.Address.fromScAddress(details.address()).toString()
              }, createV2 && {
                constructorArgs: inner.constructorArgs().map(function (arg) {
                  return (0, _scval.scValToNative)(arg);
                })
              });
              break;
            }

          // contractExecutableStellarAsset
          case 1:
            output.args.type = 'sac';
            output.args.asset = _asset.Asset.fromOperation(preimage.fromAsset()).toString();
            break;
          default:
            throw new Error("unknown creation type: ".concat(JSON.stringify(exec)));
        }
        break;
      }
    default:
      throw new Error("unknown invocation type (".concat(fn["switch"](), "): ").concat(JSON.stringify(fn)));
  }
  output.invocations = root.subInvocations().map(function (i) {
    return buildInvocationTree(i);
  });
  return output;
}

/**
 * @callback InvocationWalker
 *
 * @param {xdr.SorobanAuthorizedInvocation} node  the currently explored node
 * @param {number} depth  the depth of the tree this node is occurring at (the
 *    root starts at a depth of 1)
 * @param {xdr.SorobanAuthorizedInvocation} [parent]  this `node`s parent node,
 *    if any (i.e. this doesn't exist at the root)
 *
 * @returns {boolean|null|void}   returning exactly `false` is a hint to stop
 *    exploring, other values are ignored
 */

/**
 * Executes a callback function on each node in the tree until stopped.
 *
 * Nodes are walked in a depth-first order. Returning `false` from the callback
 * stops further depth exploration at that node, but it does not stop the walk
 * in a "global" view.
 *
 * @param {xdr.SorobanAuthorizedInvocation} root  the tree to explore
 * @param {InvocationWalker} callback  the callback to execute for each node
 * @returns {void}
 */
function walkInvocationTree(root, callback) {
  walkHelper(root, 1, callback);
}
function walkHelper(node, depth, callback, parent) {
  if (callback(node, depth, parent) === false /* allow void rv */) {
    return;
  }
  node.subInvocations().forEach(function (i) {
    return walkHelper(i, depth + 1, callback, node);
  });
}