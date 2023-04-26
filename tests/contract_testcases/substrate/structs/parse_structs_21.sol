struct A { B b; F f; }
struct B { C c; }
struct C { D d; }
struct D { A a; }
struct F { G g; }
struct G { H h; }
struct H { A a; }
// ----
// error (7-8): struct 'A' has infinite size
// 	note (11-14): recursive field 'b'
// 	note (16-19): recursive field 'f'
// error (30-31): struct 'B' has infinite size
// 	note (34-37): recursive field 'c'
// error (48-49): struct 'C' has infinite size
// 	note (52-55): recursive field 'd'
// error (66-67): struct 'D' has infinite size
// 	note (70-73): recursive field 'a'
// error (84-85): struct 'F' has infinite size
// 	note (88-91): recursive field 'g'
// error (102-103): struct 'G' has infinite size
// 	note (106-109): recursive field 'h'
// error (120-121): struct 'H' has infinite size
// 	note (124-127): recursive field 'a'
