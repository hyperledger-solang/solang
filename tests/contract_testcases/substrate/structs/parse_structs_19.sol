struct A { B b; }
struct B { A a; }
// ----
// error (7-8): struct 'A' has infinite size
// 	note (11-14): recursive field 'b'
// error (25-26): struct 'B' has infinite size
// 	note (29-32): recursive field 'a'
