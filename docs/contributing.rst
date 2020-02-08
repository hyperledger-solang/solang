Contributing
============

Solang is in active development, so there are many ways in which you can
contribute.

Target Specific Tests
---------------------

Solang supports Substrate, Burrow and ewasm. All these targets need testing
via integration tests. New targets like
`Fabric <https://github.com/hyperledger-labs/fabric-chaincode-wasm>`_ and
`Sawtooth Sabre <https://github.com/hyperledger/sawtooth-sabre>`_ need to be
added, and tests added.

How to report issues
--------------------

Please report issues to
`github issues <https://github.com/hyperledger-labs/solang/issues>`_.

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
