
contract BeingBuilt1 {
    @seed(my_seed)
    @space(1024)
    @payer(clock)
    constructor(bytes my_seed) {}
}

contract BeingBuilt2 {
    @seed(my_seed)
    @space(1024)
    @payer(systemProgram)
    constructor(bytes my_seed) {}
}

contract BeingBuilt3 {
    @seed(my_seed)
    @space(1024)
    @payer(associatedTokenProgram)
    constructor(bytes my_seed) {}
}

contract BeingBuilt4 {
    @seed(my_seed)
    @space(1024)
    @payer(rent)
    constructor(bytes my_seed) {}
}

contract BeingBuilt5 {
    @seed(my_seed)
    @space(1024)
    @payer(tokenProgram)
    constructor(bytes my_seed) {}
}

contract BeingBuilt6 {
    @seed(my_seed)
    @space(1024)
    @payer(dataAccount)
    constructor(bytes my_seed) {}
}

contract BeingBuilt7 {
    @seed(my_seed)
    @space(1024)
    @payer(SysvarInstruction)
    constructor(bytes my_seed) {}
}

contract BeingBuilt8 {
    @seed(my_seed)
    @space(1024)
    @payer(solang)
    @payer(solang)
    constructor(bytes my_seed) {}

    function say_this(string text) public pure {
        print(text);
    }
}

// ---- Expect: diagnostics ----
// error: 5:12-17: 'clock' is a reserved account name
// error: 12:12-25: 'systemProgram' is a reserved account name
// error: 19:12-34: 'associatedTokenProgram' is a reserved account name
// error: 26:12-16: 'rent' is a reserved account name
// error: 33:12-24: 'tokenProgram' is a reserved account name
// error: 40:12-23: 'dataAccount' is a reserved account name
// error: 47:12-29: 'SysvarInstruction' is a reserved account name
// error: 55:12-18: account 'solang' already defined
// 	note 54:5-19: previous definition