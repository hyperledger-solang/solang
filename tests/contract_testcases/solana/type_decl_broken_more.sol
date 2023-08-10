import "./type_decl.sol";

function foo(Addr.X x) {}

function bar(int Addr) {}

// ---- Expect: diagnostics ----
// error: 3:14-18: 'Addr' is an user type
// warning: 5:18-22: declaration of 'Addr' shadows type
// 	note 2:6-10: previous declaration of type
// warning: 7:2-33: function can be declared 'pure'
