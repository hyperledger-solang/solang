import {
    createConnection,
    Diagnostic,
    Range,
    DiagnosticSeverity,
    InitializeRequest,
    InitializeResult,
    InitializeParams,
    DefinitionRequest,
    RequestType,
    TextDocument
} from 'vscode-languageserver';

import * as rpc from 'vscode-jsonrpc';
import { DocumentLink } from 'vscode';

let connection = createConnection(
    new rpc.StreamMessageReader(process.stdin),
    new rpc.StreamMessageWriter(process.stdout)
);

//const connection = createConnection();

//connection.console.log(`Sample server running in node ${process.version}`);

connection.onInitialize((params: InitializeParams) => {
    const result: InitializeResult = {
        capabilities: {},
    };
    return result;
});

connection.onInitialized(() => {
    connection.client.register(DefinitionRequest.type, undefined);
});

function validate(): void {
    connection.sendDiagnostics({
        uri: '1',
        version: 1,
        diagnostics: [
            Diagnostic.create(Range.create(0,0,0, 10), 'Something is wrong here', DiagnosticSeverity.Warning)
        ]
    });
}

let notif = new rpc.NotificationType<string, void>('test notif');

connection.onNotification(notif, (param: string) => {
    console.log('notified\n');
    console.log(param);
});

connection.listen();