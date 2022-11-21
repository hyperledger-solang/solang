File Structure
==============

A single Solidity source file may define multiple contracts. A contract is defined
with the ``contract`` keyword, following by the contract name and then the definition
of the contract in between curly braces ``{`` and ``}``.

.. include:: ../examples/multiple_contracts.sol
  :code: solidity

When compiling this, Solang will output contract code for both `A` and `B`, irrespective of
the name of source file. Although multiple contracts maybe defined in one solidity source
file, it might be convenient to define only single contract in each file, and keep contract
name the same as the file name (with the `.sol` extension).
