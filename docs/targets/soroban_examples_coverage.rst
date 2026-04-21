Soroban Examples Coverage
=========================

This page maps upstream examples from `stellar/soroban-examples <https://github.com/stellar/soroban-examples>`_ to documented Solang Solidity examples or coverage in this repository.

The table below only includes upstream examples for which this repository currently has a clear Solidity counterpart or nearest documented coverage example. Absence from this table does not prove that an upstream example is impossible in Solang; it means there is not yet a documented counterpart in this repository.

For the current feature-oriented support status, see :doc:`soroban_support_matrix`.

Documented Counterparts
+++++++++++++++++++++++

.. list-table::
   :header-rows: 1

   * - Upstream Rust example
     - Solang Solidity example or coverage
     - Notes
   * - `alloc <https://github.com/stellar/soroban-examples/tree/main/alloc>`_
     - `tests/soroban_testcases/alloc.rs <https://github.com/hyperledger-solang/solang/blob/main/tests/soroban_testcases/alloc.rs>`_
     - Covered by Solidity testcases for dynamic memory arrays, including vector allocation, ``push()``, iteration, and summation.
   * - `atomic_swap <https://github.com/stellar/soroban-examples/tree/main/atomic_swap>`_
     - `docs/examples/soroban/atomic_swap <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/atomic_swap>`_
     - Atomic swap between two parties, with companion token contracts.
   * - `atomic_multiswap <https://github.com/stellar/soroban-examples/tree/main/atomic_multiswap>`_
     - `docs/examples/soroban/atomic_swap <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/atomic_swap>`_ and `tests/soroban_testcases/alloc.rs <https://github.com/hyperledger-solang/solang/blob/main/tests/soroban_testcases/alloc.rs>`_
     - Closest documented Solang coverage is the atomic-swap example plus array and loop support used for batching-style logic. A dedicated standalone Solidity multiswap example is not yet present in this repository.
   * - `auth <https://github.com/stellar/soroban-examples/tree/main/auth>`_
     - `docs/examples/soroban/auth.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/auth.sol>`_
     - Simple host-managed authorization via ``requireAuth()``.
   * - `cross_contract <https://github.com/stellar/soroban-examples/tree/main/cross_contract>`_
     - `integration/soroban/caller.sol <https://github.com/hyperledger-solang/solang/blob/main/integration/soroban/caller.sol>`_ and `integration/soroban/callee.sol <https://github.com/hyperledger-solang/solang/blob/main/integration/soroban/callee.sol>`_
     - Covered in `cross_contract.spec.js <https://github.com/hyperledger-solang/solang/blob/main/integration/soroban/cross_contract.spec.js>`_.
   * - `deep_contract_auth <https://github.com/stellar/soroban-examples/tree/main/deep_contract_auth>`_
     - `docs/examples/soroban/deep_auth <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/deep_auth>`_
     - Nested contract authorization via ``authAsCurrContract(...)``.
   * - `hello_world <https://github.com/stellar/soroban-examples/tree/main/hello_world>`_
     - `integration/soroban/callee.sol <https://github.com/hyperledger-solang/solang/blob/main/integration/soroban/callee.sol>`_
     - Closest local Solidity counterpart for a minimal callable contract. Solang does not currently ship a string-vector hello-world example with the same interface.
   * - `increment <https://github.com/stellar/soroban-examples/tree/main/increment>`_
     - `integration/soroban/counter.sol <https://github.com/hyperledger-solang/solang/blob/main/integration/soroban/counter.sol>`_
     - Closest local counterpart for a stored counter that can be incremented.
   * - `liquidity_pool <https://github.com/stellar/soroban-examples/tree/main/liquidity_pool>`_
     - `docs/examples/soroban/liquidity_pool <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/liquidity_pool>`_
     - Liquidity-pool and token-swap example with companion token contracts.
   * - `logging <https://github.com/stellar/soroban-examples/tree/main/logging>`_
     - `docs/examples/soroban/error.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/error.sol>`_
     - Demonstrates ``print()``-based runtime logging in Solang.
   * - `timelock <https://github.com/stellar/soroban-examples/tree/main/timelock>`_
     - `docs/examples/soroban/timelock <https://github.com/hyperledger-solang/solang/tree/main/docs/examples/soroban/timelock>`_
     - Timelock-style example using enums, mappings, authorization, and ``block.timestamp``.
   * - `token <https://github.com/stellar/soroban-examples/tree/main/token>`_
     - `docs/examples/soroban/token.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/token.sol>`_
     - Token-style contract with balances, allowances, and Soroban auth.
   * - `ttl <https://github.com/stellar/soroban-examples/tree/main/ttl>`_
     - `docs/examples/soroban/ttl_storage.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/ttl_storage.sol>`_
     - Extending TTL on stored contract data.

Solidity Translations
+++++++++++++++++++++

The following abridged snippets show how selected upstream Soroban examples are expressed in Solang Solidity.

auth
^^^^

Upstream Soroban example: `auth <https://github.com/stellar/soroban-examples/tree/main/auth>`_

Solang Solidity example: `docs/examples/soroban/auth.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/auth.sol>`_

.. code-block:: solidity

    contract auth {
        address public owner =
            address"GDRIX624OGPQEX264NY72UKOJQUASHU3PYKL6DDPGSTWXWJSBOTR6N7W";

        uint64 public instance counter = 20;

        function increment() public returns (uint64) {
            owner.requireAuth();
            counter = counter + 1;
            return counter;
        }
    }

token
^^^^^

Upstream Soroban example: `token <https://github.com/stellar/soroban-examples/tree/main/token>`_

Solang Solidity example: `docs/examples/soroban/token.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/token.sol>`_

.. code-block:: solidity

    contract token {
        address public admin;
        mapping(address => int128) public balances;

        constructor(address _admin, string memory _name, string memory _symbol, uint32 _decimals) {
            admin = _admin;
        }

        function mint(address to, int128 amount) public {
            require(amount >= 0, "Amount must be non-negative");
            admin.requireAuth();
            balances[to] = balances[to] + amount;
        }

        function transfer(address from, address to, int128 amount) public {
            from.requireAuth();
            balances[from] = balances[from] - amount;
            balances[to] = balances[to] + amount;
        }
    }

timelock
^^^^^^^^

Upstream Soroban example: `timelock <https://github.com/stellar/soroban-examples/tree/main/timelock>`_

Solang Solidity example: `docs/examples/soroban/timelock/timelock.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/timelock/timelock.sol>`_

.. code-block:: solidity

    contract timelock {
        enum TimeBoundKind { Before, After }

        struct TimeLock {
            TimeBoundKind kind;
            uint64 bound_timestamp;
            address claimant;
            uint64 amount;
        }

        mapping(address => TimeLock) public timelocks;

        function is_claimable(address claimant) public view returns (bool) {
            TimeLock storage tl = timelocks[claimant];
            return block.timestamp >= tl.bound_timestamp;
        }
    }

ttl
^^^

Upstream Soroban example: `ttl <https://github.com/stellar/soroban-examples/tree/main/ttl>`_

Solang Solidity example: `docs/examples/soroban/ttl_storage.sol <https://github.com/hyperledger-solang/solang/blob/main/docs/examples/soroban/ttl_storage.sol>`_

.. code-block:: solidity

    contract ttl_storage {
        uint64 public persistent pCount = 11;
        uint64 temporary tCount = 7;
        uint64 instance iCount = 3;

        function extend_persistent_ttl() public view returns (int64) {
            return pCount.extendTtl(1000, 5000);
        }

        function extend_temp_ttl() public view returns (int64) {
            return tCount.extendTtl(3000, 7000);
        }
    }

Upstream Examples Not Yet Documented as Supported
+++++++++++++++++++++++++++++++++++++++++++++++++

The following upstream examples do not currently have a documented Solidity counterpart, as some needed Soroban features are not yet supported.
- `bls_signature <https://github.com/stellar/soroban-examples/tree/main/bls_signature>`_
- `custom_types <https://github.com/stellar/soroban-examples/tree/main/custom_types>`_
- `deployer <https://github.com/stellar/soroban-examples/tree/main/deployer>`_
- `errors <https://github.com/stellar/soroban-examples/tree/main/errors>`_
- `eth_abi <https://github.com/stellar/soroban-examples/tree/main/eth_abi>`_
- `events <https://github.com/stellar/soroban-examples/tree/main/events>`_
- `fuzzing <https://github.com/stellar/soroban-examples/tree/main/fuzzing>`_
- `merkle_distribution <https://github.com/stellar/soroban-examples/tree/main/merkle_distribution>`_
- `mint-lock <https://github.com/stellar/soroban-examples/tree/main/mint-lock>`_
- `other_custom_types <https://github.com/stellar/soroban-examples/tree/main/other_custom_types>`_
- `privacy-pools <https://github.com/stellar/soroban-examples/tree/main/privacy-pools>`_
- `simple_account <https://github.com/stellar/soroban-examples/tree/main/simple_account>`_
- `single_offer <https://github.com/stellar/soroban-examples/tree/main/single_offer>`_
- `upgradeable_contract <https://github.com/stellar/soroban-examples/tree/main/upgradeable_contract>`_
- `workspace <https://github.com/stellar/soroban-examples/tree/main/workspace>`_

Want to add support for one of the remaining examples? Open a pull request against `hyperledger-solang/solang <https://github.com/hyperledger-solang/solang>`_ and follow the `contribution guide <https://github.com/hyperledger-solang/solang/blob/main/CONTRIBUTING.md>`_.
