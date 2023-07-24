import {User} from "./user.sol";

contract c {
    function foo(User memory user) public {
        user.clear_count();
    }
}
