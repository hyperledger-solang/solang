struct A { B b; }
struct B { A a; }
// ---- Expect: diagnostics ----
// error: 1:8-9: struct 'A' has infinite size
// 	note 1:12-15: recursive field 'b'
// error: 2:8-9: struct 'B' has infinite size
// 	note 2:12-15: recursive field 'a'
