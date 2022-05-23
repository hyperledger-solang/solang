Imports
=======

The ``import`` directive is used to import items from other Solidity files. This can be useful to
keep a single definition in one file, which can be used in other files. For example
you could have an interface in one source file, which several contracts implement or use
which are in other files. Solidity imports are somewhat similar to JavaScript ES6, however
there is no export statement, or default export.

The following items are always exported, which means they can be imported into
another file.

- global constants
- struct definitions
- enums definitions
- event definitions
- global functions
- contracts, including abstract contract, libraries, and interfaces

There are a few different flavours of import. You can specify if you want everything imported,
or a just a select few. You can also rename the imports. The following directive imports only
`foo` and `bar`:

.. code-block:: solidity

    import {foo, bar} from "defines.sol";

Solang will look for the file `defines.sol` in the same directory as the current file. You can specify
more directories to search with the ``--importpath`` commandline option.
Just like with ES6, ``import`` is hoisted to the top and both `foo` and `bar` are usuable
even before the ``import`` statement. It is also possible to import everything from
`defines.sol` by leaving the list out. Note that this is different than ES6, which would import nothing
with this syntax.

.. code-block:: solidity

    import "defines.sol";

Another method for locating files is using import maps. This maps the first directory
of an import path to a different location on the file system. Say you add
the command line option ``--importmap @openzeppelin=/opt/openzeppelin-contracts/contracts``, then

.. code-block:: solidity

    import "openzeppelin/interfaces/IERC20.sol";

will automatically map to `/opt/openzeppelin-contracts/contracts/interfaces/IERC20.sol`.

Everything defined in `defines.sol` is now usable in your Solidity file. However, if an item with the
same name is defined in `defines.sol` and also in the current file, you will get a warning. It is
permitted to import the same file more than once.

It is also possible to rename an import. In this case, only item `foo` will be imported, and `bar`
will be imported as `baz`. This is useful if you have already have a `bar` and you want to avoid
a naming conflict.

.. code-block:: solidity

    import {bar as baz,foo} from "defines.sol";

Rather than renaming individual imports, it is also possible to make all the items in a file
available under a special import object. In this case, the `bar` defined in `defines.sol` can is
now visible as `defs.bar`, and `foo` as `defs.foo`. As long as there is no previous item `defs`,
there can be no naming conflict.

.. code-block:: solidity

    import "defines.sol" as defs;

This also has a slightly more baroque syntax, which does exactly the same.

.. code-block:: solidity

    import * as defs from "defines.sol";

Builtin Imports
_______________

Some builtin functionality is only available after importing. The following structs
can be imported via the special import file ``solana``.

.. code-block:: solidity

    import {AccountMeta, AccountInfo} from 'solana';

Note that ``{AccountMeta, AccountInfo}`` can be omitted, renamed or imported via
import object.

.. code-block:: solidity

    // Now AccountMeta will be known as AM
    import {AccountMeta as AM} from 'solana';

    // Now AccountMeta will be available as solana.AccountMeta
    import 'solana' as solana;

.. note::

    The import file ``solana`` is only available when compiling for the Solana
    target.

.. _account_info:

Builtin AccountInfo
+++++++++++++++++++

The account info of all the accounts passed into the transaction. ``AccountInfo`` is a builtin
structure with the following fields:

address ``key``
    The address (or public key) of the account

uint64 ``lamports``
    The lamports of the accounts. This field can be modified, however the lamports need to be
    balanced for all accounts by the end of the transaction.

bytes ``data```
    The account data. This field can be modified, but use with caution.

address ``owner``
    The program that owns this account

uint64 ``rent_epoch``
    The next epoch when rent is due.

bool ``is_signer``
    Did this account sign the transaction

bool ``is_writable``
    Is this account writable in this transaction

bool ``executable``
    Is this account a program

.. _account_meta:

Builtin AccountMeta
+++++++++++++++++++

When doing an external call (aka CPI), ``AccountMeta`` specifies which accounts
should be passed to the callee.

address ``pubkey``
    The address (or public key) of the account

bool ``is_writable``
    Can the callee write to this account

bool ``is_signer``
    Can the callee assume this account signed the transaction
