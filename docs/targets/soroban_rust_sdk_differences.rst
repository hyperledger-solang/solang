Solang and Soroban Rust SDK Differences
=======================================

This page documents how Solang represents Solidity contracts on Soroban, and how that representation differs from common patterns used by the Soroban Rust SDK.

It focuses on data representation and storage layout.

For Solidity-facing language differences such as authorization syntax or Soroban storage class keywords, see :doc:`soroban_language_compatibility`.

Summary
+++++++

Solang and the Soroban Rust SDK target the same platform, but they do not necessarily use the same internal representations.

On Soroban, Solang uses a compiler-defined storage model for Solidity state. Arrays, structs, and nested values are lowered according to Solang's code generation strategy, not according to any Rust SDK storage convention. Developers should rely on documented Solang behavior and should not assume raw storage compatibility with handwritten Rust SDK contracts unless that layout is explicitly documented.

Storage Layout Overview
+++++++++++++++++++++++

Solang does not use EVM slot packing on Soroban.
Instead, it lowers Solidity state into Soroban storage using compiler-defined keys and Soroban host values.

The right mental model is:

- Soroban storage classes choose the ledger namespace: persistent, temporary, or instance
- Solang chooses how a Solidity variable, struct field, or array element is represented within that storage
- documented Solang behavior is what developers should rely on, not assumptions about a Rust SDK contract's internal layout

Arrays in Storage
+++++++++++++++++

For arrays of native value types, Solang uses a ``VecObject``-backed storage model.

In current Soroban code generation, storage array operations:

- load the stored ``VecObject`` handle from contract storage
- apply the relevant vector operation
- write the resulting handle back to storage

This is closer to Soroban's host vector model than to EVM-style contiguous slot arithmetic.

Relevant examples and implementation references:

- `atomic_swap <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/atomic_swap>`_
- `liquidity_pool <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/liquidity_pool>`_
- `src/emit/soroban/target.rs <https://github.com/hyperledger-solang/solang/blob/main/src/emit/soroban/target.rs>`_

Structs in Storage
++++++++++++++++++

Struct storage is also Soroban-specific.

Rather than packing a whole struct into a single EVM-like storage region, Solang encodes struct fields as Soroban values and stores fields separately under composite keys derived from the struct's storage slot and field position.

As a consequence:

- reading a single field can be relatively direct
- reading a whole struct requires loading each stored field and reconstructing the struct in memory
- full-struct loads are more expensive than field-level access

Relevant examples:

- `timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_
- `liquidity_pool.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/liquidity_pool/liquidity_pool.sol>`_

Nested Values and Composite Keys
++++++++++++++++++++++++++++++++

Arrays of custom types, including arrays of structs, follow a sparse storage approach.

The important point is that nested values are not stored like flat EVM slot ranges. Instead, the storage key tracks access structure. In current Solang Soroban support, that means:

- the first part of the key identifies the Solidity storage slot
- later parts of the key identify field positions or indexes within the nested access path

This layout is chosen to make nested mutation practical on Soroban, but it also means developers should not assume Rust SDK-like or EVM-like key shapes unless Solang explicitly documents them.

What Developers Should Not Assume
+++++++++++++++++++++++++++++++++

When comparing Solang with handwritten Rust SDK contracts, developers should not assume:

- identical raw storage keys
- identical internal layouts for arrays, structs, or nested values
- safe direct reads or writes across Solidity and Rust contracts at the raw storage level unless the layout is explicitly documented

For the current target status and support matrix, see :doc:`soroban_support_matrix`.
