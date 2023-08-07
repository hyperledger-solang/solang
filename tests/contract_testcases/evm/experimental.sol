pragma experimental solidity;

import std.stub;
import * as foo from a.b;
import {a as b} from x;

// ---- Expect: diagnostics ----
// error: 1:1-29: experimental solidity features are not supported
// error: 3:8-16: experimental import paths not supported
// error: 4:22-25: experimental import paths not supported
// error: 5:22-23: experimental import paths not supported
