struct A { B b; }
struct B { A[] a; mapping(uint=>A) m; }
struct C { B b; D d; }
struct D { uint e; }
abstract contract Foo {}

// ---- Expect: diagnostics ----
