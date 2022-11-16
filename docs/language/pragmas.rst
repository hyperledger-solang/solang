Pragmas
=======

A pragma value is a special directive to the compiler. It has a name, and a value. The name
is an identifier and the value is any text terminated by a semicolon `;`. Solang parses
pragmas but does not recognise any.

Often, Solidity source files start with a ``pragma solidity`` which specifies the Ethereum
Foundation Solidity compiler version which is permitted to compile this code. Solang does
not follow the Ethereum Foundation Solidity compiler version numbering scheme, so these
pragma statements are silently ignored. There is no need for a ``pragma solidity`` statement
when using Solang.

.. code-block:: solidity

    pragma solidity >=0.4.0 <0.4.8;
    pragma experimental ABIEncoderV2;

The `ABIEncoderV2` pragma is not needed with Solang; structures can always be ABI encoded or
decoded. All other pragma statements are ignored, but generate warnings.

About pragma solidity versions
------------------------------

Ethereum Solidity checks the value of ``pragma version`` against the compiler version, and
gives an error if they do not match. Ethereum Solidity is often revising the language
in various small ways which make versions incompatible which other. So, the
version pragma ensures that the compiler version matches what the author of the
contract was using, and ensures the compiler will give no unexpected errors.

Solang takes a different view:

#. Solang tries to remain compatible with different versions of ethereum solidity;
   we cannot publish a version of solang for every version of the ethereum solidity
   compiler.
#. We also have compatibility issues because we target multiple blockchains, so
   the version would not be sufficient.
#. We think that the compiler version should not be a property of the source,
   but of the build environment. No other language set the compiler version in
   the source code.

If anything, some languages allow conditional compilation based on the compiler
version, which is much more useful.
