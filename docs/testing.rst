Solang Test Suite
=================

Solang has a few test suites. These are all run on each pull request via
`github actions <https://github.com/hyperledger-solang/solang/actions>`_.


Solidity parser and semantics tests
-----------------------------------

In the `tests <https://github.com/hyperledger-solang/solang/tree/main/tests>`_ directory, there are
a lot of tests which call `fn parse_and_resolve()`. This function parses Solidity, and returns
the *namespace*: all the resolved contracts, types, functions, etc (as much as could be resolved),
and all the compiler diagnostics, i.e. compiler warnings and errors. These tests check that
the compiler parser and semantic analysis work correctly.

Note that Solidity can import other solidity files using the ``import`` statement. There are further
tests which create a file cache with filenames and their contents, to ensure that imports
work as expected.


Codegen tests
-------------

The stage after semantic analysis is codegen. Codegen generates an IR which is a CFG, so it is
simply called CFG. The codegen tests ensure that the CFG matches what should be created. These
tests are inspired by LLVM lit tests. The tests can found in
`codegen_testcases <https://github.com/hyperledger-solang/solang/tree/main/tests/codegen_testcases>`_.

These tests do the following:

 - Look for a comment ``// RUN:`` and then run the compiler with the given arguments and the filename itself
 - After that the output is compared against special comments:
 - ``// CHECK:`` means that following output must be present
 - ``// BEGIN-CHECK:`` means check for the following output but scan the output from the beginning
 - ``// FAIL:`` will check that the command will fail (non-zero exit code) with the following output

Mock contract virtual machine
-----------------------------

For Polkadot and Solana there is a mock virtual machine. System and runtime call
implementations should semantically represent the real on-chain virtual machine as exact as
possible. Aspects that don't matter in the context of unit testing (e.g. gas-metering) may be
ignored in the mock virtual machine. For Polkadot, this uses the
`wasmi crate <https://crates.io/crates/wasmi>`_ and for Solana it
uses the `Solana RBPF crate <https://crates.io/crates/solana_rbpf>`_.

These tests consist of calling a function call `fn build_solidity()` which compiles the given
solidity source code and then returns a `VM`. This `VM` can be used to deploy one
of the contract, and test various functions like contract storage, accessing builtins such as
block height, or creating/calling other contracts. Since the functionality is mocked, the test
can do targeted introspection to see if the correct function was called, or walk the heap
of the contract working memory to ensure there are no corruptions.


Deploy contract on dev chain
----------------------------

There are some tests in `integration <https://github.com/hyperledger-solang/solang/tree/main/integration/>`_
which are written in node. These tests start an actual real chain via containers,
and then deploying some tests contracts to them and interacting with them.