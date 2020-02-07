Contributing
============

Solang is in active development, so there are many ways in which you can
contribute.

Target Specific Tests
---------------------

Solang supports Substrate, Burrow and ewasm. All these targets need testing
via integration tests. New targets like
`Farbic <https://github.com/hyperledger-labs/fabric-chaincode-wasm>`_ and
`Sawtooth Sabre <https://github.com/hyperledger/sawtooth-sabre>`_ need to be
added, and tests added.

How to report issues
--------------------

Please report issues to
`github issues <https://github.com/hyperledger-labs/solang/issues>`_.

Style guide
-----------

Solang follows default rustfmt, and clippy. Any clippy warnings need to be fixed.
Outside of the tests, code should ideally be written in a such a way that no
``#[allow(clippy::foo)]`` are needed.

For test code, this is much less strict. It is much more important that tests are
written, and that they have good coverage rather than worrying about clippy warnings.
Feel free to sprinkle some ``#[allow(clippy::foo)]`` around your test code to make
your merge request pass.
