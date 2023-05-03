function foo(int) {}
struct S { int f1; }
function bar(S memory) {}

using {foo} for *;
using {foo} for int global;
using {foo} for int;
using {bar} for S meh;

function test(int a) {
	a.foo();
}

contract c {
	function f(S memory s) public {
		s.bar();
	}
}


// ---- Expect: diagnostics ----
// error: 5:1-18: using must be bound to specific type, '*' cannot be used on file scope
// error: 6:21-27: 'global' only permitted on user defined types
// error: 8:19-22: 'meh' not expected, did you mean 'global'?
