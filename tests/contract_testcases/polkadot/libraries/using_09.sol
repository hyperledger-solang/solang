function foo(int) {}
struct S { int f1; }
function bar(S memory) {}

using {foo} for int;
using {bar} for S global;

function test(int a) {
	a.foo();
}

contract c {
	function f(S memory s) public {
		s.bar();
	}
}


// ---- Expect: diagnostics ----
// warning: 1:1-19: function can be declared 'pure'
// warning: 3:1-24: function can be declared 'pure'
