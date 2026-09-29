Soroban Support Matrix
======================

This page is the documentation for feature support status on the Soroban target.

This support matrix documents the Soroban features Solang currently intends to support. It should be read as a statement of documented target behavior, not as a claim of exhaustive coverage. Stronger completeness claims would require broader automated validation, such as fuzzing and differential testing, and that work is still in progress.

The Soroban target as a whole is still pre-alpha and experimental.

Related documentation:

.. list-table::
   :header-rows: 1

   * - Page
     - Purpose
   * - :doc:`soroban_examples_coverage`
     - Upstream `stellar/soroban-examples` coverage and the corresponding Solang Solidity examples.
   * - :doc:`soroban_language_compatibility`
     - Solidity-facing differences and Soroban-specific language behavior.
   * - :doc:`soroban_rust_sdk_differences`
     - Storage layout, host-value representation, and differences from common Rust SDK patterns.

Language Features
+++++++++++++++++

.. list-table::
   :header-rows: 1

   * - Feature area
     - Status
     - Details and examples
   * - Contract model
     - Supported
     - Constructors with arguments, public functions, and public getters. External functions return at most one value; Solidity's multiple-return-value tuples are not supported as external return types. Examples: `token.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/token.sol>`_, `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_, and `storage_types.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/storage_types.sol>`_.
   * - Core types and collections
     - Supported
     - Primitive types, ``uint256`` and ``int256``, ``string``, ``bytes``, ``bytesN``, mappings and nested mappings, and memory/storage arrays, including fixed-size ``T[N]`` arrays, nested arrays, and arrays of structs (``S[]``). Examples: `token.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/token.sol>`_, `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_, `liquidity_pool.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/liquidity_pool/liquidity_pool.sol>`_, `atomic_swap <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/atomic_swap>`_, and `liquidity_pool <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/liquidity_pool>`_.
   * - Events and logs
     - Supported
     - Solidity ``event`` declarations and ``emit`` statements are supported. Indexed fields map to Soroban event topics and non-indexed fields map to event data. ``print()`` and runtime error logging are also available. Example: `error.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/error.sol>`_.
   * - Structs
     - Partial support
     - Structs work as ABI function parameters and return values, and can be read from and written to contract storage. Fields of any scalar or reference type are supported, including ``string``, ``bytes``, ``bytesN``, nested structs, and integers up to 256-bit. Struct fields may also be dynamic arrays such as ``bytes[]``, and arrays of structs (``S[]``) work as function parameters and return values. Example using a struct with a dynamic-array field: `groth16_verifier.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/groth16_verifier.sol>`_. Structs are not yet supported as ``public`` state-variable accessor return values or as ``event`` declaration parameters.
   * - Enums and other complex user-defined types
     - Partial support
     - Enumerations are supported. Examples: `custom_types.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/custom_types.sol>`_ and `other_custom_types.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/other_custom_types.sol>`_. Coverage of deeper composite values is still limited and some combinations may not compile.
   * - Yul and inline assembly
     - Unsupported
     - Not supported on the Soroban target.
   * - Hash and cryptographic builtins
     - Supported
     - ``sha256`` and ``keccak256`` hashing, the BLS12-381 builtins ``bls12_381_g1_add``, ``bls12_381_g1_mul``, and ``bls12_381_pairing_check``, and XDR serialization via ``to_xdr``. Examples: `groth16_verifier.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/groth16_verifier.sol>`_ (BLS12-381 Groth16 proof verification) and `merkle_distribution.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/merkle_distribution.sol>`_ (``sha256`` and ``to_xdr`` Merkle proofs).

The exact Solidity support boundary can only be characterized with broader fuzzing and related validation, and that work is still in progress.

If you run into a missing Solidity feature on Soroban, please `open an issue <https://github.com/hyperledger-solang/solang/issues/new/choose>`_.

Soroban Features
++++++++++++++++

.. list-table::
   :header-rows: 1

   * - Feature area
     - Status
     - Details and examples
   * - Authorization
     - Supported
     - ``address.requireAuth()``, ``address.requireAuthForArgs(...)`` (scopes the authorization to an explicit argument list, mapping to the host's ``require_auth_for_args``), and ``auth.authAsCurrContract(...)``. Examples: `auth.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/auth.sol>`_, `token.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/token.sol>`_, `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_, `mint_lock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/mint_lock.sol>`_, and `deep_auth <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/deep_auth>`_.
   * - Cross-contract calls
     - Supported
     - ``address.call(...)`` and the documented ABI encode/decode flows around it. Examples: `deep_auth <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/deep_auth>`_ and `cross_contract.spec.js <https://github.com/hyperledger-solang/solang/blob/main/integration/soroban/cross_contract.spec.js>`_.
   * - Storage classes and TTL
     - Supported
     - Storage classes ``persistent``, ``temporary``, and ``instance``, plus ``extendTtl(...)`` and ``extendInstanceTtl(...)``. Examples: `storage_types.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/storage_types.sol>`_ and `ttl_storage.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/ttl_storage.sol>`_.
   * - Soroban utilities
     - Supported
     - ``block.timestamp`` and ``block.number`` (the current ledger sequence number). Examples: `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_ (``block.timestamp``) and `mint_lock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/mint_lock.sol>`_ (``block.number``).
   * - Deploying and upgrading contracts
     - Partial support
     - Programmatic deployment via the ``deployContract(bytes32 wasm_hash, bytes32 salt, ...args)`` builtin, which deploys an already-uploaded Wasm blob (by hash) on behalf of the current contract and runs its constructor, mapping to the host function ``create_contract_with_constructor``. Example: `deployer <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/deployer>`_. Live contract upgrades are supported through the ``updateCurrentContractWasm(bytes32 new_wasm_hash)`` builtin, which replaces the running contract's Wasm with an already-uploaded blob (host function ``update_current_contract_wasm``). Example: `upgradeable_contract.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/upgradeable_contract.sol>`_. Solidity's ``new Contract()`` syntax (deploying a compile-time-known contract) is not yet supported.
   * - Native value transfer and payable-style flows
     - Unsupported
     - This is not part of the documented Solang support surface on Soroban.
   * - ``selfdestruct``
     - Unsupported
     - Not supported on the Soroban target.

Where a feature has target-specific behavior rather than being simply supported or unsupported, that behavior is documented in the compatibility pages linked above.
