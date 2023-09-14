
contract BeingBuilt1 {
    @space(1024)
    @payer(clock)
    constructor(@seed bytes my_seed) {}
}

contract BeingBuilt2 {
    @space(1024)
    @payer(systemProgram)
    constructor(@seed bytes my_seed) {}
}

contract BeingBuilt3 {
    @space(1024)
    @payer(associatedTokenProgram)
    constructor(@seed bytes my_seed) {}
}

contract BeingBuilt4 {
    @space(1024)
    @payer(rent)
    constructor(@seed bytes my_seed) {}
}

contract BeingBuilt5 {
    @space(1024)
    @payer(tokenProgram)
    constructor(@seed bytes my_seed) {}
}

contract BeingBuilt6 {
    @space(1024)
    @payer(dataAccount)
    constructor(@seed bytes my_seed) {}
}

contract BeingBuilt7 {
    @payer(SysvarInstruction)
    constructor(@seed bytes my_seed, @space uint64 my_space) {}
}

contract BeingBuilt8 {
    @seed("pine_tree")
    @space(1024)
    @payer(solang)
    @payer(solang)
    constructor() {}

    function say_this(string text) public pure {
        print(text);
    }
}


contract OldAnnotationSyntax {
    @payer(my_account)
    @seed(my_seed)
    @space(my_space)
    @bump(my_bump)
    constructor(bytes my_seed, uint64 my_space, bytes1 my_bump) {}

    function my_func(@myNote uint64 a) public pure returns (uint64) {
        return a-2;
    }
}

// ---- Expect: diagnostics ----
// error: 4:12-17: 'clock' is a reserved account name
// error: 10:12-25: 'systemProgram' is a reserved account name
// error: 16:12-34: 'associatedTokenProgram' is a reserved account name
// error: 22:12-16: 'rent' is a reserved account name
// error: 28:12-24: 'tokenProgram' is a reserved account name
// error: 34:12-23: 'dataAccount' is a reserved account name
// error: 39:12-29: 'SysvarInstruction' is a reserved account name
// error: 47:12-18: account 'solang' already defined
// 	note 46:5-19: previous definition
// error: 58:11-18: '@seed' annotation on a constructor only accepts constant values
// error: 59:12-20: '@space' annotation on a constructor only accepts constant values
// error: 60:11-18: '@bump' annotation on a constructor only accepts constant values
// error: 63:22-29: parameter annotations are only allowed in constructors