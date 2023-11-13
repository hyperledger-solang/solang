pragma foo bar;
pragma abicode v2;
pragma abicode "v2";
pragma solidity ^0.5.16 0.4 - 1 =0.8.22 || >=0.8.21 <=2 ~1 0.6.2;

contract C {}

// ---- Expect: dot ----
// ---- Expect: diagnostics ----
// warning: 1:1-15: unknown pragma 'foo' with value 'bar' ignored
// warning: 2:1-18: unknown pragma 'abicode' with value 'v2' ignored
// warning: 3:1-20: unknown pragma 'abicode' with value 'v2' ignored
