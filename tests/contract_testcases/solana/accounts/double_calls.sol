contract adult {
    hatchling hh;
    function test() external {
        hatchling h1 = new hatchling("luna");
        hatchling h2 = new hatchling("sol");
    }

    function create(string id) external {
        hh = new hatchling(id);
    }

    function call2() external {
        hh.call_me("id");
        hh.call_me("not_id");
    }

    function create2(string id2) external {
        hh = new hatchling(id2);
        hh.call_me(id2);
    }
}


@program_id("5kQ3iJ43gHNDjqmSAtE1vDu18CiSAfNbRe4v5uoobh3U")
contract hatchling {
    string name;

    constructor(string id) payable {
        require(id != "", "name must be provided");
        name = id;
    }

    function call_me(string name2) view external {
        if (name != name2) {
                print("This is not my name");
        } else {
                print("Have I heard my name?");
        }
    }
}

// ---- Expect: diagnostics ----
// warning: 4:19-21: local variable 'h1' is unused
// warning: 5:19-21: local variable 'h2' is unused
// error: 5:24-44: contract 'hatchling' is called more than once in this function, so automatic account collection cannot happen. Please, provide the necessary accounts using the {accounts:..} call argument
// 	note 4:24-45: other call
// warning: 12:5-30: function can be declared 'view'
// error: 14:9-29: contract 'hatchling' is called more than once in this function, so automatic account collection cannot happen. Please, provide the necessary accounts using the {accounts:..} call argument
// 	note 13:9-25: other call
// error: 19:9-24: contract 'hatchling' is called more than once in this function, so automatic account collection cannot happen. Please, provide the necessary accounts using the {accounts:..} call argument
// 	note 18:14-32: other call
