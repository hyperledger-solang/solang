# Hover for solang-vscode
Hover in solang-vscode can be used to quickly check the variable types
of the respective variables for hassle free programming.

### Hover can be experienced when you:
- Open a solidity file(.sol).
- Place/Hover the mouse(pointer) over a variable name.

### Wondering how this works?
When you point at a position, the client sends the cursor position to the
server. The server converts line, char to respective file offsets.
The server looks for the particular position inside its lookup table
which has pre-computed hover messages for the respective left-right ranges
of file offsets. After locating the respective messages it is rendered
back to the client as a new Hover object.

Before starting to process the hover requests from the client, the server
computes a tuple array of (left-offset, right-offset, message) by traversing
the ast statements followed by expressions and stores each variable values/types
in the lookup table along with the respective messages.

### Which properties are supported:
1. Variables types in the enums, structs, functions, contracts.

### Want to run some tests?
Currently there are 3 test cases running over hover1.sol file in src/test/testfixture.

### How to run these tests?
1. Build the extension (Ctrl+Shift+B).
2. Press F5 and from the bottom menu select "Extension tests".
The result should appear in the debug console.

The server might take a few seconds to respond during file changes.
We are working on more features, stay tuned!