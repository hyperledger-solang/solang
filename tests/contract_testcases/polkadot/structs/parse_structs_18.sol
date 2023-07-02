struct A { B b; }
struct B { A a; mapping(uint=>A) m; }
struct C { B b; D d; }
struct D { uint e; }
abstract contract Foo {}
// ---- Expect: diagnostics ----
// error: 1:8-9: struct 'A' has infinite size
// 	note 1:12-15: recursive field 'b'
// error: 2:8-9: struct 'B' has infinite size
// 	note 2:12-15: recursive field 'a'
// 	note 2:17-35: recursive field 'm'
// error: 3:8-9: struct 'C' has infinite size
// 	note 3:12-15: recursive field 'b'
