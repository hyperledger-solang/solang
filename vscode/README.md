# Solang Solidity Compiler

The Solang Solidity Compiler compiles Solidity for the following blockchains:

- Parity Substrate
- Solana
- evm (enough for the extension)

This extension provides syntax highlighting, diagnostics  (i.e. compiler warnings and errors), and information on types and functions when hovering.

Both the Visual Studio Code extension code and language server were developed under a
[Hyperledger Mentorship programme](https://wiki.hyperledger.org/display/INTERN/Create+a+new+Solidity+Language+Server+%28SLS%29+using+Solang+Compiler).

For more information on Solang itself and the extension, please go to the [documentation](https://solang.readthedocs.io/en/latest/).

## Dependencies

The solang compiler executable needs to be installed, which can be downloaded from
the [Solang Releases Page](https://github.com/hyperledger-labs/solang/releases). Then
you have to configure the path to the solang executable extension settings, and also
which target you wish to compile file.

Please see the [extension documentation](https://solang.readthedocs.io/en/latest/extension.html).

## References

Files solidity.configuration.json, syntaxes/solidity.json reffered from https://github.com/juanfranblanco/vscode-solidity

Commit hash: e22c566909a18ae646cbc41ea3e788222c8377a6

The MIT License (MIT)

Copyright (c) 2016 Juan Blanco

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
