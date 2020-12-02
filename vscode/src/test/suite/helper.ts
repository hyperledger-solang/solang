import * as vscode from 'vscode';
import * as path from 'path';

export let doc: vscode.TextDocument;
export let editor: vscode.TextEditor;
export let documentEol: string;
export let platformEol: string;

export async function activate(docUri: vscode.Uri) {
	// The extensionId is `publisher.name` from package.json

	let ext: vscode.Extension<any> | undefined = vscode.extensions.getExtension('vscode.slang-ex');

	let extn_act;

	if(ext){
	try {
		extn_act = await ext.activate();
	}
	catch(e){
		console.error(e);
	}
	}
	else{
		console.error('extension is undefined');
	}

	try {
		doc = await vscode.workspace.openTextDocument(docUri);
		editor = await vscode.window.showTextDocument(doc);
		await sleep(5000);
	} catch (e) {
		console.error(e);
	}
}

async function sleep(ms: number) {
	return new Promise(resolve => setTimeout(resolve, ms));
}

export const getDocPath = (p: string) => {
	return path.resolve(__dirname, '../../../src/testFixture', p);
};

export const getDocUri = (p: string) => {
	return vscode.Uri.file(getDocPath(p));
};

export async function setTestContent(content: string): Promise<boolean> {
	const all = new vscode.Range(
		doc.positionAt(0),
		doc.positionAt(doc.getText().length)
	);
	return editor.edit(eb => eb.replace(all, content));
}

export async function getedits(){
	const chang = doc.lineAt(0);

	return chang;
}