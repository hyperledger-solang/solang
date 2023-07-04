
.. _yul_builtins:

Builtins
========

Most operations in Yul are performed via builtin functions. Solang supports
most builtins, however memory operations and chain operations are not implemented.
Yul builtins are low level instructions and many are `ethereum specific <https://ethereum.org/en/developers/docs/evm/opcodes/>`_.
On Solana and Polkadot, some builtins, like ``delegatecall`` and ``staticcall``, for instance, are not available
because the concept they implement does not exist in neither chains.

.. warning::
    In addition to nonexistent builtins, due to low-level differences between
    blockchain virtual machines, it is impossible to replicate the builtin's behavior outside Ethereum. ``pop``, for example,
    removes an item from the stack in EVM, however, in Solana there is no stack, for its virtual machine is register based.

This is the comprehensive list of the existing Yul builtins and their compatibility on Solang. Arithmetic operations
always return the widest integer between the arguments. Signed numbers are represented in two's complement. The
descriptions in the table have been slightly modified from the `Solc documentation <https://docs.soliditylang.org/en/latest/yul.html#evm-dialect>`_.


+-------------------------+-------------+-------------------------------------------+-----------------+
| Builtin                 | Returns     | Explanation                               | Availability    |
+=========================+=============+===========================================+=================+
| stop()                  | None        | stop execution                            | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| add(x, y)               | Integer     | x + y                                     | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| sub(x, y)               | Integer     | x - y                                     | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| mul(x, y)               | Integer     | x * y                                     | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| div(x, y)               | Integer     | x / y or 0 if y == 0                      | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| sdiv(x, y)              | Integer     | x / y or 0 if y == 0, for signed integers | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| mod(x, y)               | Integer     | x % y or 0 if y == 0                      | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| smod(x, y)              | Integer     | x % y or 0 if y == 0, for signed integers | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| exp(x, y)               | Integer     | x to the power of y                       | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| not(x)                  | Integer     | negation of every bit of x                | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| lt(x, y)                | Bool        | x < y                                     | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| gt(x, y)                | Bool        | x > y                                     | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| slt(x, y)               | Bool        | x < y, for signed integers                | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| sgt(x, y)               | Bool        | x > y, for signed integers                | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| eq(x, y)                | Bool        | x == y                                    | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| iszero(x)               | Bool        | x == 0                                    | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| and(x, y)               | Integer     | bitwise AND of x and y                    | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| or(x, y)                | Integer     | bitwise OR of x and y                     | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| xor(x, y)               | Integer     | bitwise XOR of x and y                    | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| byte(n, x)              | Integer     | nth byte of x, where 0 is the most        | Yes             |
|                         |             | significant bit                           |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| shl(x, y)               | Integer     | y << x                                    | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| shr(x, y)               | Integer     | y >> x                                    | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| sar(x, y)               | Integer     | y >> x, for signed integers               | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| addmod(x, y, m)         | Integer     | (x + y) % m or 0 if m == 0                | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| mulmod(x, y, m)         | Integer     | (x * y) % m or 0 if m == 0                | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| signextend(i, x)        | Integer     | | sign extend from (i*8+7)th bit, where   | No              |
|                         |             | | 0th is the least significant bit        |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| keccak256(p, n)         | Integer     | keccak(mem[p...(p+n)))                    | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| pc()                    | Integer     | program counter                           | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| pop(x)                  | None        | discard value x from the stack            | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| mload(p)                | Integer     | load from memory mem[p...(p+32))          | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| mstore(p, v)            | None        | store v in memory mem[p...(p+32))         | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| mstore8(p, v)           | None        | store v & 0xff byte in memory mem[p]      | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| sload(p)                | Integer     | Load from storage slot p                  | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| sstore(p, v)            | Integer     | store v in storage slot p                 | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| msize()                 | Integer     | largest accessed memory index             | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| gas()                   | Integer     | gas still available to execution          | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| address()               | Integer     | address of the current contract           | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| balance(a)              | Integer     | balance at address a                      | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| selfbalance()           | Integer     | equivalent to balance(address())          | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| caller()                | Integer     | call sender (excluding ``delegatecall``)  | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| callvalue()             | Integer     | wei sent together with the current call   | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| calldataload(p)         | Integer     | load call data starting from position p   | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| calldatasize()          | Integer     | size of call data in bytes                | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| calldatacopy(t, f, s)   | None        | | copy s bytes from calldata at position  | No              |
|                         |             | | f to mem at position t                  |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| codesize()              | Integer     | | size of the code of the current         | No              |
|                         |             | | contract or execution context           |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| codecopy(t, f, s)       | None        | | copy s bytes from code at position f    | No              |
|                         |             | | to mem at position t                    |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| extcodesize(a)          | Integer     | size of the code at address a             | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| extcodecopy(a, t, f, s) | None        | | like codecopy(t, f, s),                 | No              |
|                         |             | | but take code at address a              |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| returndatasize()        | Integer     | size of the last returndata               | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| returndatacopy(t, f, s) | None        | | copy s bytes from returndata at         | No              |
|                         |             | | position f to mem at position t         |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| extcodehash(a)          | Integer     | code hash of address a                    | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| create(v, p, n)         | Integer     | | create new contract with code           | No              |
|                         |             | | mem[p...(p+n)) and send v wei and       |                 |
|                         |             | | return the new address; returns 0       |                 |
|                         |             | | on error                                |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| create2(v, p, n, s)     | Integer     | | create new contract with code           | No              |
|                         |             | | mem[p...(p+n)) at address resulting     |                 |
|                         |             | | from the keccak256 hash of              |                 |
|                         |             | | 0xff.this.s.keccak256(mem[p..(p+n)])    |                 |
|                         |             | | and send v wei and return the new       |                 |
|                         |             | | address, where ``0xff`` is a 1 byte     |                 |
|                         |             | | value, ``this`` is the current          |                 |
|                         |             | | contract's address as a 20 byte value   |                 |
|                         |             | | and ``s`` is a big-endian 256-bit       |                 |
|                         |             | | value; returns 0 on error               |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| | call(g, a, v, in,     | Integer     | | call contract at address a with in      | No              |
| | insize, out, outsize) |             | | mem[in...(in+insize)) providing g gas   |                 |
|                         |             | | and v wei and output area               |                 |
|                         |             | | mem[out...(out+outsize)) returning 0    |                 |
|                         |             | | on error (eg. out of gas) and 1 on      |                 |
|                         |             | | success                                 |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| | callcode(g, a, v, in, | Integer     | | identical to ``call`` but only use the  | No              |
| | insize, out, outsize) |             | | code from a and stay in the context of  |                 |
|                         |             | | the current contract otherwise          |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| | delegatecall(g, a, in,| Integer     | | identical to ``callcode`` but also keep | No              |
| | insize, out, outsize) |             | | ``caller`` and ``callvalue``            |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| | staticcall(g, a, in,  | Integer     | | identical to ``call`` but do not allow  | No              |
| | insize, out, outsize) |             | | state modifications                     |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| return(p, s)            | None        | end execution, return data mem[p...(p+s)) | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| revert(p, s)            | None        | | end execution, revert state changes,    | No              |
|                         |             | | return data mem[p...(p+s))              |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| selfdestruct(a)         | None        | | end execution, destroy current          | No              |
|                         |             | | contract and send funds to a            |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| invalid()               | None        | end execution with invalid instruction    | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| log0(p, s)              | None        | log without topics and data mem[p...(p+s)]| No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| log1(p, s, t1)          | None        | log with topic t1 and data mem[p...(p+s)] | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| log2(p, s, t1, t2)      | None        | | log with topics t1, t2 and data         | No              |
|                         |             | | mem[p...(p+s))                          |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| log3(p, s, t1, t2, t3)  | None        | | log with topics t1, t2, t3 and data     | No              |
|                         |             | | mem[p...(p+s))                          |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| | log4(p, s, t1, t2, t3,| None        | | log with topics t1, t2, t3, t4 and      | No              |
| | t4)                   |             | | data mem[p...(p+s))                     |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| chainid()               | Integer     | ID of the executing chain                 | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| basefee()               | Integer     | current block's base fee                  | No              |
+-------------------------+-------------+-------------------------------------------+-----------------+
| origin()                | Integer     | transaction sender                        | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| gasprice()              | Integer     | gas price of the transaction              | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| blockhash(b)            | Integer     | | hash of block nr b - only for last      | Yes             |
|                         |             | | 256 blocks excluding current            |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| coinbase()              | Integer     | current mining beneficiary                | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| timestamp()             | Integer     | | timestamp of the current block in       | Yes             |
|                         |             | | seconds since the epoch                 |                 |
+-------------------------+-------------+-------------------------------------------+-----------------+
| number()                | Integer     | current block number                      | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| difficulty()            | Integer     | difficulty of the current block           | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+
| gaslimit()              | Integer     | block gas limit of the current block      | Yes             |
+-------------------------+-------------+-------------------------------------------+-----------------+