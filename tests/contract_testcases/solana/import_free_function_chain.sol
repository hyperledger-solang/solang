// ensure free function can be imported via chain
import "import_free_function.sol" as Y;

function baz() {
	int x = Y.X.foo();
}

// ----
// warning (0-28): function can be declared 'pure'
