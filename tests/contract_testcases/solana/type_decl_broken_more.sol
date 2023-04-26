import "type_decl.sol";

function foo(Addr.X x) {}

function bar(int Addr) {}

// ----
// error (38-42): 'Addr' is an user type
// warning (69-73): declaration of 'Addr' shadows type
// 	note (6-10): previous declaration of type
// warning (69-100): function can be declared 'pure'
