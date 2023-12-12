pragma foo bar;
pragma abicoder v2;
pragma solidity ^4294967296;
pragma solidity 0 - 1 0 - 2;
pragma abicoder "v2";
pragma solidity ^0.5.16 =0.8.22 || >=0.8.21 <=2 ~1 0.6.2;
pragma solidity 0.4 - 1 || 0.3 - 0.5.16;

contract C {
	enum E { a, b, c }
	uint8 constant maxE = type(E).max;
}

// ---- Expect: dot ----
// ---- Expect: diagnostics ----
// error: 1:1-15: unknown pragma 'foo' with value 'bar'
// error: 3:17-28: '4294967296' is not a valid number
// error: 4:1-28: version ranges can only be combined with the || operator
