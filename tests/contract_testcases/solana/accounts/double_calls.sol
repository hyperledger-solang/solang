contract adult {
    function test() external {
        hatchling.new("luna");
        hatchling.new("sol");
    }

    function create(string id) external {
        hatchling.new(id);
    }

    function call2() external {
        hatchling.call_me("id");
        hatchling.call_me("not_id");
    }

    function create2(string id2) external {
        hatchling.new(id2);
        hatchling.call_me(id2);
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
// error: 4:9-29: contract 'hatchling' is called more than once in this function, so automatic account collection cannot happen. Please, provide the necessary accounts using the {accounts:..} call argument
// 	note 3:9-30: other call
// warning: 11:5-30: function can be declared 'view'
// error: 13:9-36: contract 'hatchling' is called more than once in this function, so automatic account collection cannot happen. Please, provide the necessary accounts using the {accounts:..} call argument
// 	note 12:9-32: other call
// error: 18:9-31: contract 'hatchling' is called more than once in this function, so automatic account collection cannot happen. Please, provide the necessary accounts using the {accounts:..} call argument
// 	note 17:9-27: other call
