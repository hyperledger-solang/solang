struct S {
    int256 f1;
    S[] f2;
}

function foo(S memory s) pure {}

// FIXME (stack overrun in emit):
// contract Foo {
//	function bar() public {
//		S memory s = S({ f1: 1, f2: new S[](0) });
//		foo(s);
//	}
// }

// ---- Expect: diagnostics ----
