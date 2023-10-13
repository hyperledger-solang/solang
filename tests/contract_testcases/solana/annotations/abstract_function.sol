
abstract contract Base1 {
    @account(acc1)
    function test(uint64 val) virtual external {}
}

abstract contract Base2 {
    @mutableAccount(acc2)
    function test(uint64 val) virtual external {}
}

contract Derived1 is Base1 {
    bool b;
    @signer(other)
    function test(uint64 val) override (Base1) external {
        b = (tx.accounts.dataAccount.key == address(this));
    }
}

contract Derived2 is Base1, Base2 {
    bool b;
    @account(acc1)
    function test(uint64 val) override(Base1, Base2) external {
        b = true;
    }
}

// ---- Expect: diagnostics ----
// error: 4:5-47: functions must have the same declared accounts for correct overriding
// 	note 3:5-19: corresponding account 'acc1' is missing
// 	note 8:5-26: corresponding account 'acc2' is missing
// error: 15:5-56: functions must have the same declared accounts for correct overriding
// 	note 14:5-19: corresponding account 'other' is missing
// 	note 3:5-19: corresponding account 'acc1' is missing
// error: 23:5-62: functions must have the same declared accounts for correct overriding
// 	note 22:5-19: corresponding account 'acc1' is missing
// 	note 8:5-26: corresponding account 'acc2' is missing
