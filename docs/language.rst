Solidity Language
=================

The Solidity language support by Solang is compatible with the
`Ethereum Foundation Solidity Compiler <https://github.com/ethereum/solidity/>`_ with
these caveats:

- At this point solang is very much a work in progress so not at all features
  are supported yet.

- Solang can target different blockchains and depending on the target. For example,
  Parity Substrate uses a different ABI encoding and allows constructors to be
  overloaded.

- Solang generates WebAssembly rather than EVM. This means that the ``assembly {}``
  using EVM instructions is not supported.

.. note::

  Where differences exist between different targets or the Ethereum Foundation Solidity
  compiler, this is noted in boxes like these.

Types
-----

The following primitive types are supported:

``bool``
  This represents a single value which can be either ``true`` or ``false``

``uint``
  This represents a single unsigned integer of 256 bits wide. Values can be for example
  ``0``, ``102``, ``0xdeadcafe``, or ``1000_000_000_000_000``.

``uint64``, ``uint32``, ``uint16``, ``uint8``
  These represent shorter single unsigned integers of the given width. These widths are
  most efficient in WebAssembly and should be used whenever possible.

``uintN``
  These represent shorter single unsigned integers of width ``N``. ``N`` can be anything
  between 8 and 256 bits.

``int``
  This represents a single signed integer of 256 bits wide. Values can be for example
  ``-102``, ``0``, ``102`` or ``-0xdead_cafe``.

``int64``, ``uint32``, ``uint16``, ``uint8``
  These represent shorter single signed integers of the given width. These widths are
  most efficient in WebAssembly and should be used whenever possible.

``intN``
  These represent shorter single signed integers of width ``N``. ``N`` can be anything
  between 8 and 256 bits.

Underscores ``_`` are allowed in numbers, as long as the number does not start with
an underscore.

Assigning values which cannot fit into the type gives a compiler error. For example::

    uint8 foo = 300;

The largest value an ``uint8`` can hold is (2^8) - 1 = 255. So, the compiler says::

    implicit conversion would truncate from uint16 to uint8

.. tip::

  Whenever possible use the the ``int64``, ``int32`` or ``uint64``, ``uint32`` types.

  The Solidity language has its origins in the Ethereum Virtual Machine (EVM), which has
  support for 256 bit registers. Most common CPUs like x86_64 do not implement arithmetic
  for such large types, and the EVM virtual machine itself has to do bigint calculations, which
  are costly. This means that EVM instructions with gas cost of 1 can be very expensive in
  real CPU cost.

  WebAssembly does not support this. This means that Solang has to emulate larger types with
  multiple WebAssembly instructions, resulting in larger contract code and higher gas cost.
  As a result, gas cost approximates real CPU cost much better.

