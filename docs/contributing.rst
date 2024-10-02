Contributing
============

Solang is in active development, so there are many ways in which you can
contribute.

* Consider that users who will read the docs are from different background and cultures and that they have different preferences.
* Avoid potential offensive terms and, for instance, prefer "allow list and deny list" to "white list and black list".
* We believe that we all have a role to play to improve our world, and even if writing inclusive doc might not look like a huge improvement, it's a first step in the right direction.
* We suggest to refer to `Microsoft bias free writing guidelines <https://docs.microsoft.com/en-us/style-guide/bias-free-communication>`_
  and `Google inclusive doc writing guide <https://developers.google.com/style/inclusive-documentation>`_ as starting points.

How to report issues
--------------------

Please report issues to
`github issues <https://github.com/hyperledger-solang/solang/issues>`_.

How to contribute code
----------------------

Code contributions are submitted via 
`pull requests <https://github.com/hyperledger-solang/solang/compare>`_.

Please fork this repository and make desired changes inside a dedicated branch on your fork.
Prior to opening a pull request for your branch, make sure that the code in your branch

* does compile without any warnings (run ``cargo build --workspace``)
* does not produce any clippy lints (run ``cargo clippy --workspace``)
* does pass all unit tests (run ``cargo test --workspace``)
* has no merge conflicts with the ``main`` branch
* is correctly formatted (run ``cargo fmt --all`` if your IDE does not do that automatically)

Target Specific
---------------

Solang supports Polkadot and Solana. These targets need testing
via integration tests. New targets like
`Fabric <https://github.com/hyperledger-solang/fabric-chaincode-wasm>`_ need to be
added, and tests added.

Debugging issues with LLVM
--------------------------

The Solang compiler builds `LLVM IR <http://releases.llvm.org/8.0.1/docs/LangRef.html>`_.
This is done via the `inkwell <https://github.com/TheDan64/inkwell>`_ crate, which is
a "safe" rust wrapper. However, it is easy to construct IR which is invalid. When this
happens you might get segfaults deep in llvm. There are two ways to help when this
happens.

Build LLVM with Assertions Enabled
__________________________________

If you are using llvm provided by your distribution, llvm will not be build with
``LLVM_ENABLE_ASSERTIONS=On``. See :ref:`llvm-from-source` how to build
your own.

Verify the IR with llc
______________________

Some issues with the IR will not be detected even with LLVM Assertions on. These includes
issues like instructions in a basic block after a branch instruction (i.e. unreachable
instructions).

Run ``solang --emit llvm -v foo.sol`` and you will get a foo.ll file, assuming that the
contract in foo.sol is called foo. Try to compile this with ``llc foo.ll``. If IR is
not valid, llc will tell you.

Style guide
-----------

Solang follows default rustfmt, and clippy. Any clippy warnings need to be fixed.
Outside of the tests, code should ideally be written in a such a way that no
``#[allow(clippy::foo)]`` are needed.

For test code, this is much less strict. It is much more important that tests are
written, and that they have good coverage rather than worrying about clippy warnings.
Feel free to sprinkle some ``#[allow(clippy::foo)]`` around your test code to make
your merge request pass.
