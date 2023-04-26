struct A { B b; }
struct B { A a; mapping(uint=>A) m; }
struct C { B b; D d; }
struct D { uint e; }
abstract contract Foo {}
// ----
// error (7-8): struct 'A' has infinite size
// 	note (11-14): recursive field 'b'
// error (25-26): struct 'B' has infinite size
// 	note (29-32): recursive field 'a'
// 	note (34-52): recursive field 'm'
// error (63-64): struct 'C' has infinite size
// 	note (67-70): recursive field 'b'
