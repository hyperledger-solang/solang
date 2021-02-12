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
  window,
  extensions,
  WorkspaceFolder
} from 'vscode';

import {
  LANGUAGE_SERVER_ID,
  LANGUAGE_SERVER_NAME,
  DEFAULT_SOLANG_BIN_PATH,
  CONFIG_KEY_COMMAND,
  CONFIG_KEY_TARGET
} from './constants';

import { getGithubReleases } from './installer';
import {
  expandPathTilde,
  getDefaultProperties,
  getConfigValueOrThrow
} from './util';

/**
 * Called upon extension activation.
 *
 * Enforces presence of required config values, and sets up the language
 * server & commands for managment of local solang binaries.
 *
 * @TODO: Doc
 * @TODO: Add UI component
 */
const activate = async (context: vscode.ExtensionContext) :Promise<void> => {
  // const releases = await getGithubReleases();
  // console.log(JSON.stringify(releases, null, 2));

  const extension :vscode.Extension<any> = extensions.getExtension(LANGUAGE_SERVER_ID);
  const command :string = await getConfigValueOrThrow(CONFIG_KEY_COMMAND, context);
  const target :string = await getConfigValueOrThrow(CONFIG_KEY_TARGET, context);

  context.subscriptions.push(
    vscode.languages.createDiagnosticCollection(LANGUAGE_SERVER_ID)
  );

  const connection = rpc.createMessageConnection(
    new rpc.StreamMessageReader(process.stdout),
    new rpc.StreamMessageWriter(process.stdin)
  );

  connection.listen();

  const sop: Executable = {
    command: expandPathTilde(command),
    args: ['--language-server', '--target', target],
  };

  const serverOptions: ServerOptions = sop;
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { language: 'solidity', scheme: 'file' },
      { language: 'solidity', scheme: 'untitled' },
    ]
  };

  // TODO: Remove? Unused, TBD at completion
  // const init: InitializeParams = {
  // 	rootUri: null,
  // 	processId: 1,
  // 	capabilities: {},
  // 	workspaceFolders: null,
  // };
  //
  // const params = {
  // 	textDocument: { uri: 'file://temp' },
  // 	position: { line: 1, 'character': 1 }
  // };

  const solangClient = new LanguageClient(
    LANGUAGE_SERVER_ID,
    LANGUAGE_SERVER_NAME,
    serverOptions,
    clientOptions
  ).start();

  context.subscriptions.push(solangClient);

  window.showInformationMessage(`Congratulations, your extension "${LANGUAGE_SERVER_NAME}" is now active!`);
};

export default activate;
