Soroban Solidity Language Compatibility
=======================================

This page documents the Solidity-facing compatibility expectations for the Soroban target.
It focuses on source-level language behavior: how Solidity constructs map to Soroban, where behavior differs from normal EVM expectations, and which Soroban-specific constructs developers should expect to use.

This page is about language compatibility, not storage-layout differences. For Solang-specific storage representation and differences from common Rust SDK patterns, see :doc:`soroban_rust_sdk_differences`.

Source-Level Compatibility
++++++++++++++++++++++++++

Solang aims for source-level Solidity familiarity on Soroban, not EVM equivalence.

In practice, that means:

- familiar Solidity syntax should compile where the Soroban backend supports it
- Solang may warn on Soroban-specific constraints rather than silently changing behavior
- when Soroban cannot support a construct safely, Solang should reject it rather than compile incorrect semantics

The current supported and unsupported feature set is tracked on :doc:`soroban_support_matrix`.

Authorization Model
+++++++++++++++++++

The clearest Solidity-language difference on Soroban is authorization.

On EVM chains, access control is often written in terms of ``msg.sender``. Soroban does not expose authorization through ``msg.sender`` in the same way. Instead, authorization is performed by the Soroban host for a specific ``address``.

An EVM-style ownership check often looks like this:

.. code-block:: solidity

    pragma solidity ^0.8.20;

    contract OnlyOwner {
        address owner;

        constructor() {
            owner = msg.sender;
        }

        function set(uint256 value) public {
            require(msg.sender == owner, "Only owner can call this function");
        }
    }

On Soroban, the same intent should be written using ``requireAuth()`` on the address that must authorize the call:

.. code-block:: solidity

    contract auth {
        address public owner;
        uint64 public counter;

        constructor(address _owner) public {
            owner = _owner;
        }

        function increment() public returns (uint64) {
            owner.requireAuth();
            counter = counter + 1;
            return counter;
        }
    }

The ``requireAuth()`` builtin calls into the Soroban host, which verifies that the address authorized the invocation. This is the Soroban-native pattern developers should expect to use instead of direct ``msg.sender`` comparisons.

For deeper authorization chains that pass through another contract, Solang also supports ``auth.authAsCurrContract(...)``. See:

- the `deep auth example <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/deep_auth>`_
- `docs/examples/soroban/auth.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/auth.sol>`_
- `tests/soroban_testcases/auth.rs <https://github.com/hyperledger-solang/solang/blob/main/tests/soroban_testcases/auth.rs>`_

Storage Classes and Lifetime
++++++++++++++++++++++++++++

Soroban has `three storage types: Persistent, Temporary and Instance <https://developers.stellar.org/docs/build/guides/storage/choosing-the-right-storage>`_.
Solang exposes these directly in the language with Soroban-only storage class keywords:

- ``persistent``
- ``temporary``
- ``instance``

For example:

.. code-block:: solidity

    contract storage_types {
        uint64 public temporary var = 1;
        uint64 public instance var1 = 1;
        uint64 public persistent var2 = 2;
        uint64 public var3 = 2;

        function inc() public {
            var++;
            var1++;
            var2++;
            var3++;
        }
    }

If no storage class is written, the variable defaults to persistent storage.

This is a Soroban-specific language extension rather than standard Solidity behavior. Developers should treat it as part of the Soroban target surface, not portable Solidity syntax.

Related Soroban-only lifetime helpers are available through builtins such as ``extendTtl(...)`` and ``extendInstanceTtl(...)``. See :doc:`../language/builtins` and:

- `docs/examples/soroban/storage_types.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/storage_types.sol>`_
- `docs/examples/soroban/ttl_storage.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/ttl_storage.sol>`_
- `tests/soroban_testcases/ttl.rs <https://github.com/hyperledger-solang/solang/blob/main/tests/soroban_testcases/ttl.rs>`_

Types, Structs, Arrays, and Mappings
++++++++++++++++++++++++++++++++++++

The Soroban backend supports a growing subset of Solidity types and compound constructs, including the tested use of:

- primitive types such as ``bool``, ``address``, ``string``, ``uint32``, ``uint64``, ``int32``, ``int64``, ``int128``, ``uint128``, ``uint256``, and ``int256``
- structs in storage
- mappings and nested mappings
- memory arrays and storage arrays

These are Solidity-facing language features, but some of their behavior is constrained by Soroban's host representation and storage model.
For the storage and representation side of those features, see :doc:`soroban_rust_sdk_differences`.

Current documented support is summarized on :doc:`soroban_support_matrix`.

Integer Widths
++++++++++++++

Soroban only natively supports integer widths ``32``, ``64``, ``128``, and ``256``.
When a contract uses another integer width, Solang rounds the width up to the next supported Soroban size:

- ``1..=32`` becomes ``32``
- ``33..=64`` becomes ``64``
- ``65..=128`` becomes ``128``
- ``129..=256`` becomes ``256``

By default, this produces a warning. If you compile with ``--strict-soroban-types``, the same cases become compilation errors instead.

This is another case where Solang preserves Solidity source compatibility where practical, but makes the Soroban-specific constraint visible rather than silently pretending arbitrary integer widths are native to the runtime.

Current Expectation
+++++++++++++++++++

When writing Solidity for Soroban, developers should expect:

- Solidity-like syntax where supported
- explicit Soroban-specific language differences where the runtime requires them
- warnings or rejections instead of silent mismatches

For the current support matrix, see :doc:`soroban_support_matrix`.
