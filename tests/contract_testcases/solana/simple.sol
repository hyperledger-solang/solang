contract A {}
library L {}
error E(int foo);
struct S {
	int64 f1;
	bool f2;
}
function inc(S s) pure { s.f1 += 1; }
using {inc} for S global;

// ---- Expect: diagnostics ----
// warning: 3:7-8: error 'E' has never been used
