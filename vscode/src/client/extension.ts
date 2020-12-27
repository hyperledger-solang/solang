// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from 'vscode';
import * as path from 'path';
import { homedir } from 'os';
import * as cp from 'child_process';
import * as rpc from 'vscode-jsonrpc';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind,
	InitializeRequest,
	InitializeParams,
	DefinitionRequest,
	Executable,
	ExecutableOptions
} from 'vscode-languageclient';

import {
	workspace,
	WorkspaceFolder
} from 'vscode';


let diagcollect: vscode.DiagnosticCollection;


function expandPathResolving(path: string) {
	if (path.startsWith('~/')) {
		return path.replace('~', homedir());
	}
	return path;
}

// this method is called when your extension is activated
// your extension is activated the very first time the command is executed
export function activate(context: vscode.ExtensionContext) {

	const config = workspace.getConfiguration('solang');

	let command: string = config.get('languageServerExecutable') || '~/.cargo/bin/solang';
	let target: string = config.get('target') || 'substrate';

	// Use the console to output diagnostic information (console.log) and errors (console.error)
	// This line of code will only be executed once when your extension is activated
	console.log('Congratulations, your extension "solang" is now active!');

	diagcollect = vscode.languages.createDiagnosticCollection('solidity');

	context.subscriptions.push(diagcollect);

	let connection = rpc.createMessageConnection(
		new rpc.StreamMessageReader(process.stdout),
		new rpc.StreamMessageWriter(process.stdin)
	);

	connection.listen();

	const sop: Executable = {
		command: expandPathResolving(command),
		args: ['--language-server', '--target', target],
	};

	const serverOptions: ServerOptions = sop;

	const clientoptions: LanguageClientOptions = {
		documentSelector: [
			{ language: 'solidity', scheme: 'file' },
			{ language: 'solidity', scheme: 'untitled' },
		]
	};

	const init: InitializeParams = {
		rootUri: null,
		processId: 1,
		capabilities: {},
		workspaceFolders: null,
	};

	const params = {
		"textDocument": { "uri": "file://temp" },
		"position": { "line": 1, "character": 1 }
	};


	let clientdispos = new LanguageClient(
		'solidity',
		'Solang Solidity Compiler',
		serverOptions,
		clientoptions).start();

	context.subscriptions.push(clientdispos);
}

// this method is called when your extension is deactivated
export function deactivate() { }
