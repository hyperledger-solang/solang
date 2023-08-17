contract FooFoo {
    @payer(dataAccount)
    constructor() {

    }
}

contract FooBar {
    @payer(my_dataAccount)
    constructor() {

    }
}

contract BarFoo {
    @payer(otherdataAccount)
    constructor() {
        
    }
}


// ---- Expect: diagnostics ----
// error: 2:12-23: 'dataAccount' is a reserved account name
// error: 9:12-26: account names that contain 'dataAccount' are reserved
// error: 16:12-28: account names that contain 'dataAccount' are reserved
