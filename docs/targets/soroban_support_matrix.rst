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
     - Constructors with arguments, public functions, and public getters. Examples: `token.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/token.sol>`_, `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_, and `storage_types.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/storage_types.sol>`_.
   * - Core types and collections
     - Supported
     - Primitive types, ``uint256`` and ``int256``, documented struct shapes, mappings and nested mappings, and memory/storage arrays. Examples: `token.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/token.sol>`_, `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_, `liquidity_pool.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/liquidity_pool/liquidity_pool.sol>`_, `atomic_swap <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/atomic_swap>`_, and `liquidity_pool <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/liquidity_pool>`_.
   * - Events and logs
     - Partially supported
     - ``print()`` and runtime error logging are available. Solidity events are not. Example: `error.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/error.sol>`_.
   * - Complex user-defined and composite types
     - Partial support
     - Coverage is still limited. More complex user-defined types and deeper composite values, such as nested structs, might not compile in some cases.
   * - Yul and inline assembly
     - Unsupported
     - Not supported on the Soroban target.
   * - Solidity hash and crypto builtins
     - Unsupported
     - Not yet supported on the Soroban target.

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
     - ``address.requireAuth()`` and ``auth.authAsCurrContract(...)``. Examples: `auth.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/auth.sol>`_, `token.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/token.sol>`_, `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_, and `deep_auth <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/deep_auth>`_.
   * - Cross-contract calls
     - Supported
     - ``address.call(...)`` and the documented ABI encode/decode flows around it. Examples: `deep_auth <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/deep_auth>`_ and `cross_contract.spec.js <https://github.com/hyperledger-solang/solang/blob/main/integration/soroban/cross_contract.spec.js>`_.
   * - Storage classes and TTL
     - Supported
     - Storage classes ``persistent``, ``temporary``, and ``instance``, plus ``extendTtl(...)`` and ``extendInstanceTtl(...)``. Examples: `storage_types.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/storage_types.sol>`_ and `ttl_storage.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/ttl_storage.sol>`_.
   * - Soroban utilities
     - Supported
     - ``block.timestamp``. Example: `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_.
   * - Creating contracts with ``new``
     - Unsupported
     - Contract creation from Solidity is not supported on Soroban.
   * - Native value transfer and payable-style flows
     - Unsupported
     - This is not part of the documented Solang support surface on Soroban.
   * - ``selfdestruct``
     - Unsupported
     - Not supported on the Soroban target.

Where a feature has target-specific behavior rather than being simply supported or unsupported, that behavior is documented in the compatibility pages linked above.
