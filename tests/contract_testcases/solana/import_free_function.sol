// ensure free function can be imported (plain, renamed, import symbol)
import "for_if_no_else.sol";
import {foo as renamed_foo} from "for_if_no_else.sol";
import * as X from "for_if_no_else.sol";

function bar() {
	int x = foo();
	x = renamed_foo();
	x = X.foo();
	x = X.foo({});
}

// ----
// warning (0-28): function can be declared 'pure'
