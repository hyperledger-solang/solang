/**
 * @title Hello, World!
 * @notice Just an example.
 * @author Sean Young <sean@mess.org>
 */
contract c {
    /// @param name The name which will be greeted
    function say_hello(string name) public {
        print(string.concat("Hello, ", name, "!"));
    }
}
