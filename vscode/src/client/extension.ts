// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from 'vscode';

import * as path from 'path';

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

// this method is called when your extension is activated
// your extension is activated the very first time the command is executed
export function activate(context: vscode.ExtensionContext) {

	// Use the console to output diagnostic information (console.log) and errors (console.error)
	// This line of code will only be executed once when your extension is activated
	console.log('Congratulations, your extension "solang" is now active!');

	diagcollect = vscode.languages.createDiagnosticCollection('solidity');

	context.subscriptions.push(diagcollect);

	// The command has been defined in the package.json file
	// Now provide the implementation of the command with registerCommand
	// The commandId parameter must match the command field in package.json
	let disposable = vscode.commands.registerCommand('solang.helloWorld', () => {
		// The code you place here will be executed every time your command is executed

		// Display a message box to the user
		vscode.window.showInformationMessage('Hello World from solang!');
	});

	context.subscriptions.push(disposable);

	let connection = rpc.createMessageConnection(
		new rpc.StreamMessageReader(process.stdout),
		new rpc.StreamMessageWriter(process.stdin)
	);


	connection.listen();

	const sop: Executable = {
		command: 'solang --language-server',
		options: {
			shell: true
		}
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
		'Soliditiy language server extension',
		serverOptions,
		clientoptions).start();

	context.subscriptions.push(clientdispos);


	let disposable1 = vscode.commands.registerCommand('solang.sendfirstcode', () => {
		connection.sendRequest(DefinitionRequest.type, params);
		console.log('sent request\n');
	});
	context.subscriptions.push(disposable1);

	let disposable2 = vscode.commands.registerCommand('solang.applyedit', () => {

		const { activeTextEditor } = vscode.window;

		if (activeTextEditor && activeTextEditor.document.languageId === 'solidity') {
			const { document } = activeTextEditor;
			const frst = document.lineAt(0);

			if (frst.text !== '42') {
				const edit = new vscode.WorkspaceEdit();
				edit.insert(document.uri, frst.range.start, '42\n');
				console.log('sent edit request\n');

				return vscode.workspace.applyEdit(edit);
			}
		}
	});
	context.subscriptions.push(disposable2);

}

// this method is called when your extension is deactivated
export function deactivate() { }
