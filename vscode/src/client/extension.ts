// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from 'vscode';
import * as rpc from 'vscode-jsonrpc';
import { promises as fs } from 'fs';
import { join } from 'path';
import { LanguageClient, LanguageClientOptions, ServerOptions, Executable } from 'vscode-languageclient';
import expandPathResolving from '../utils/expandPathResolving';
import getServer from '../utils/getServer';

// this method is called when your extension is activated
// your extension is activated the very first time the command is executed
export async function activate(context: vscode.ExtensionContext) {
  await tryActivate(context).catch((err) => {
    void vscode.window.showErrorMessage(`Cannot activate solang: ${err.message}`);
    throw err;
  });
}

async function tryActivate(context: vscode.ExtensionContext) {
  await fs.mkdir(context.globalStoragePath, { recursive: true });

  const path = await bootstrapServer(context);
  await bootstrapExtension(context, path);
}

async function bootstrapExtension(context: vscode.ExtensionContext, serverpath: string) {
  const config = vscode.workspace.getConfiguration('solang');
  const target: string = config.get('target') || 'substrate';

  // Use the console to output diagnostic information (console.log) and errors (console.error)
  // This line of code will only be executed once when your extension is activated
  console.log('Congratulations, your extension "solang" is now active!');

  const diagnosticCollection = vscode.languages.createDiagnosticCollection('solidity');

  context.subscriptions.push(diagnosticCollection);

  const connection = rpc.createMessageConnection(
    new rpc.StreamMessageReader(process.stdout),
    new rpc.StreamMessageWriter(process.stdin)
  );

  connection.listen();

  const sop: Executable = {
    command: expandPathResolving(serverpath),
    args: ['--language-server', '--target', target],
  };

  const serverOptions: ServerOptions = sop;

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { language: 'solidity', scheme: 'file' },
      { language: 'solidity', scheme: 'untitled' },
    ],
  };

  const client = new LanguageClient('solidity', 'Solang Solidity Compiler', serverOptions, clientOptions).start();

  context.subscriptions.push(client);
}

async function bootstrapServer(context: vscode.ExtensionContext) {
  let path
  if (process.env.NODE_ENV === 'test') {
    path = join(context.globalStoragePath, 'solang')
  } else {
    path = await getServer(context);
  }

  if (!path) {
    throw new Error('Solang Language Server is not available.');
  }

  console.log('Using server binary at', path);

  return path;
}
