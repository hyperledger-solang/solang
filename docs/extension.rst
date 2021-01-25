
Visual Studio Code Extension
============================

Solang has
`language server <https://en.wikipedia.org/wiki/Language_Server_Protocol>`_ built
into the executable, which can be used by the Visual Studio Code extension. This
extension provides the following:

1. Syntax highlighting
2. Compiler warnings and errors are displayed in the problems tab and marked
   with squiqqly lines, this is also known as `diagnostics`.
3. Hovering over variables, types, functions etc and more will give information,
   For example this will give the struct fields when hovering over a variable
   which is a reference to a struct.

.. image:: extension-screenshot.png

Both the Visual Studio Code extension code and the language server were developed under a
`Hyperledger Mentorship programme <https://wiki.hyperledger.org/display/INTERN/Create+a+new+Solidity+Language+Server+%28SLS%29+using+Solang+Compiler>`_.

Using the extension
-------------------

The extension can be found on the `Visual Studio Marketplace <https://marketplace.visualstudio.com/items?itemName=solang.solang>`_.

First, install the extension and the Solang compiler binary. The extension needs
to know where to find the Solang binary to start the language server, and also
it needs to know what target you wish to compile your solidity code for.

.. image:: extension-config.png

Development
-----------

The code is spread over two parts. The first part the vscode extension client code,
`written in TypeScript <https://github.com/hyperledger-labs/solang/tree/main/vscode>`_.
This part deals with syntax highlighting, and calling out to the Solang language server when
needed. The client needs `npm and node installed <https://docs.npmjs.com/downloading-and-installing-node-js-and-npm>`_.
The client implementation is present in
`src/client <https://github.com/hyperledger-labs/solang/tree/main/vscode/src/client>`_.
The extension client code is in
`src/client/extension.ts <https://github.com/hyperledger-labs/solang/tree/main/vscode/src/client/extension.ts>`_.

Secondly, there is the language server which is written in Rust.
The Solang binary has an option ``--language-server``, which start the
`built-in language server <https://github.com/hyperledger-labs/solang/blob/main/src/bin/languageserver/mod.rs>`_.

Once you have node and npm installed, you can build the extension like so:

.. code-block:: bash

    git clone https://github.com/hyperledger-labs/solang
    cd solang/vscode
    npm install
    npm install -g vsce
    vsce package

You should now have an extension file called solang-0.0.1.vsix which can be
installed using `code --install-extension solang-0.0.1.vsix`.

Alternatively, the extension be run from vscode itself.

1. Inside a vscode instance, Ctrl+Shift+B to build the project
2. On the task bar at the bottom of the IDE select Launch Client
3. Open a Solidity file (.sol) to test the extension.

To run the tests:

1. Inside a vscode instance, Ctrl+Shift+B to build the project
2. On the task bar at the bottom of the IDE select Extensions tests
3. The result should be displayed in the debug console of the host IDE instance.
