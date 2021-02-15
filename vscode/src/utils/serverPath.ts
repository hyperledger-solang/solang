import * as vscode from 'vscode';

export function getServerPath(context: vscode.ExtensionContext): string | undefined {
  return context.globalState.get('languageServerExecutable');
}

export function setServerPath(context: vscode.ExtensionContext, path: string | undefined) {
  return context.globalState.update('languageServerExecutable', path);
}
