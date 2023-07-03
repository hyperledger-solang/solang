struct A { B b; }
struct B { A[] a; }
// ---- Expect: diagnostics ----
