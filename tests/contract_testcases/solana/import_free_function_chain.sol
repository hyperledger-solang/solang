// ensure free function can be imported via chain
import "./import_free_function.sol" as Y;

function baz() {
	int x = Y.X.foo();
}

// ---- Expect: diagnostics ----
// warning: 1:1-29: function can be declared 'pure'
