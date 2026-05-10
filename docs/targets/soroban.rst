Soroban
========

.. toctree::
   :maxdepth: 1
   :hidden:

   soroban_support_matrix
   soroban_examples_coverage
   soroban_language_compatibility
   soroban_rust_sdk_differences

.. note::
   The Soroban target is still pre-alpha.

Solang can compile Solidity contracts for the Soroban smart contract platform on Stellar.
The Soroban target is already useful for a growing subset of contracts, but it is not yet feature-complete.

Documentation
+++++++++++++

The Soroban documentation is split into the following sections:

- `Soroban support matrix <soroban_support_matrix.html>`_ for the current supported and unsupported feature set
- `Soroban examples coverage <soroban_examples_coverage.html>`_ for upstream `stellar/soroban-examples` coverage and the corresponding Solang Solidity examples
- `Soroban Solidity language compatibility <soroban_language_compatibility.html>`_ for Solidity on EVM language differences i.e authorizations syntax.
- `Solang and Soroban Rust SDK differences <soroban_rust_sdk_differences.html>`_ for differences between Solang and Soroban Rust SDK i.e storage layout. 

For the upstream Rust examples themselves, see `stellar/soroban-examples <https://github.com/stellar/soroban-examples>`_.

Status
++++++

The Soroban target is experimental. It is usable for a documented subset of Solidity contracts, but it is not yet feature-complete or production-ready. See the `Soroban support matrix <soroban_support_matrix.html>`_.

Developers should rely on the support matrix and compatibility pages for the current documented behavior.

Background
++++++++++

Soroban contracts run as Wasm modules and communicate with the Soroban host through Soroban ``Val`` values and host objects.
For platform background, see `Host and Guest <https://developers.stellar.org/docs/learn/fundamentals/contract-development/environment-concepts#host-and-guest>`_ and `CAP-0046-1 <https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-01.md>`_.
