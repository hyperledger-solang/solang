struct A { B b; F f; }
struct B { C c; }
struct C { D d; }
struct D { A a; }
struct F { G g; }
struct G { H h; }
struct H { A a; }
// ---- Expect: diagnostics ----
// error: 1:8-9: struct 'A' has infinite size
// 	note 1:12-15: recursive field 'b'
// 	note 1:17-20: recursive field 'f'
// error: 2:8-9: struct 'B' has infinite size
// 	note 2:12-15: recursive field 'c'
// error: 3:8-9: struct 'C' has infinite size
// 	note 3:12-15: recursive field 'd'
// error: 4:8-9: struct 'D' has infinite size
// 	note 4:12-15: recursive field 'a'
// error: 5:8-9: struct 'F' has infinite size
// 	note 5:12-15: recursive field 'g'
// error: 6:8-9: struct 'G' has infinite size
// 	note 6:12-15: recursive field 'h'
// error: 7:8-9: struct 'H' has infinite size
// 	note 7:12-15: recursive field 'a'
