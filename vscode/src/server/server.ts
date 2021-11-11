import { createConnection, InitializeResult, DefinitionRequest } from 'vscode-languageserver';
import * as rpc from 'vscode-jsonrpc';

const connection = createConnection(
  new rpc.StreamMessageReader(process.stdin),
  new rpc.StreamMessageWriter(process.stdout)
);

connection.onInitialize(() => {
  const result: InitializeResult = {
    capabilities: {},
  };
  return result;
});

connection.onInitialized(() => {
  connection.client.register(DefinitionRequest.type, undefined);
});

const notif = new rpc.NotificationType<string, void>('test notif');

connection.onNotification(notif, (param: string) => {
  console.log('notified\n');
  console.log(param);
});

connection.listen();
