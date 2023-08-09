import * as assert from 'assert';

import * as vscode from 'vscode';
import { getDocUri, activate } from './helper';

// You can import and use all API from the 'vscode' module
// as well as import your extension to test it
// import * as myExtension from '../../extension';

suite('Extension Test Suite', function () {
  vscode.window.showInformationMessage('Start all tests.');

  this.timeout(20000);
  const diagnosdoc1 = getDocUri('one.sol');
  test('Testing for Row and Col pos.', async () => {
    await testdiagnos(diagnosdoc1, [
      {
        message: `unrecognised token 'aa', expected "(", ";", "="`,
        range: toRange(5, 0, 5, 2),
        severity: vscode.DiagnosticSeverity.Error,
        source: 'solidity',
      },
    ]);
  });

  this.timeout(20000);
  const diagnosdoc2 = getDocUri('two.sol');
  test('Testing for diagnostic errors.', async () => {
    await testdiagnos(diagnosdoc2, [
      {
        message:
          `unrecognised token '}', expected "!", "(", "+", "++", "-", "--", "[", "address", "bool", "byte", "bytes", "case", "default", "delete", "false", "function", "leave", "mapping", "new", "payable", "revert", "string", "switch", "true", "type", "~", Bytes, Int, Uint, address, hexnumber, hexstring, identifier, number, rational, string`,
        range: toRange(13, 1, 13, 2),
        severity: vscode.DiagnosticSeverity.Error,
        source: 'solidity',
      },
    ]);
  });

  this.timeout(20000);
  const diagnosdoc3 = getDocUri('three.sol');
  test('Testing for diagnostic info.', async () => {
    await testdiagnos(diagnosdoc3, []);
  });

  this.timeout(20000);
  const diagnosdoc4 = getDocUri('four.sol');
  test('Testing for diagnostics warnings.', async () => {
    await testdiagnos(diagnosdoc4, [
      {
        message: `unknown pragma 'foo' with value 'bar' ignored`,
        range: toRange(0, 0, 0, 14),
        severity: vscode.DiagnosticSeverity.Warning,
        source: `solidity`,
      },
      {
        message: `function can be declared 'pure'`,
        range: toRange(3, 5, 3, 40),
        severity: vscode.DiagnosticSeverity.Warning,
        source: `solidity`,
      },
    ]);
  });

  // Tests for hover.
  this.timeout(20000);
  const hoverdoc1 = getDocUri('hover1.sol');
  test('Testing for Hover', async () => {
    await testhover(hoverdoc1);
  });

  // Tests for goto-definitions.
  this.timeout(20000);
  const defdoc1 = getDocUri('defs.sol');
  test('Testing for GotoDefinitions', async () => {
    await testdefs(defdoc1);
  });
});

function toRange(lineno1: number, charno1: number, lineno2: number, charno2: number) {
  const start = new vscode.Position(lineno1, charno1);
  const end = new vscode.Position(lineno2, charno2);
  return new vscode.Range(start, end);
}

async function testdefs(docUri: vscode.Uri) {
  await activate(docUri);

  const pos1 = new vscode.Position(38, 16);
  const actualdef1 = (await vscode.commands.executeCommand(
    'vscode.executeDefinitionProvider',
    docUri,
    pos1
  )) as vscode.Definition[];
  const loc1 = actualdef1[0] as vscode.Location;
  assert.strictEqual(loc1.range.start.line, 27);
  assert.strictEqual(loc1.range.start.character, 24);
  assert.strictEqual(loc1.range.end.line, 27);
  assert.strictEqual(loc1.range.end.character, 25);
  assert.strictEqual(loc1.uri.path, docUri.path);

  const pos2 = new vscode.Position(33, 18);
  const actualdef2 = (await vscode.commands.executeCommand(
    'vscode.executeDefinitionProvider',
    docUri,
    pos2
  )) as vscode.Definition[];
  const loc2 = actualdef2[0] as vscode.Location;
  assert.strictEqual(loc2.range.start.line, 27);
  assert.strictEqual(loc2.range.start.character, 50);
  assert.strictEqual(loc2.range.end.line, 27);
  assert.strictEqual(loc2.range.end.character, 55);
  assert.strictEqual(loc2.uri.path, docUri.path);

  const pos3 = new vscode.Position(32, 31);
  const actualdef3 = (await vscode.commands.executeCommand(
    'vscode.executeDefinitionProvider',
    docUri,
    pos3
  )) as vscode.Definition[];
  const loc3 = actualdef3[0] as vscode.Location;
  assert.strictEqual(loc3.range.start.line, 19);
  assert.strictEqual(loc3.range.start.character, 8);
  assert.strictEqual(loc3.range.end.line, 19);
  assert.strictEqual(loc3.range.end.character, 12);
  assert.strictEqual(loc3.uri.path, docUri.path);

  const pos4 = new vscode.Position(29, 18);
  const actualdef4 = (await vscode.commands.executeCommand(
    'vscode.executeDefinitionProvider',
    docUri,
    pos4
  )) as vscode.Definition[];
  const loc4 = actualdef4[0] as vscode.Location;
  assert.strictEqual(loc4.range.start.line, 23);
  assert.strictEqual(loc4.range.start.character, 8);
  assert.strictEqual(loc4.range.end.line, 23);
  assert.strictEqual(loc4.range.end.character, 15);
  assert.strictEqual(loc4.uri.path, docUri.path);

  const pos5 = new vscode.Position(28, 14);
  const actualdef5 = (await vscode.commands.executeCommand(
    'vscode.executeDefinitionProvider',
    docUri,
    pos5
  )) as vscode.Definition[];
  const loc5 = actualdef5[0] as vscode.Location;
  assert.strictEqual(loc5.range.start.line, 24);
  assert.strictEqual(loc5.range.start.character, 8);
  assert.strictEqual(loc5.range.end.line, 24);
  assert.strictEqual(loc5.range.end.character, 14);
  assert.strictEqual(loc5.uri.path, docUri.path);
}

async function testhover(docUri: vscode.Uri) {
  await activate(docUri);

  const pos1 = new vscode.Position(74, 14);

  const actualhover = (await vscode.commands.executeCommand(
    'vscode.executeHoverProvider',
    docUri,
    pos1
  )) as vscode.Hover[];

  const contentarr1 = actualhover[0].contents as vscode.MarkdownString[];

  assert.strictEqual(contentarr1[0].value, '```solidity\nmapping(address => uint256) storage SimpleAuction.pendingReturns\n```');

  const pos2 = new vscode.Position(78, 19);

  const actualhover2 = (await vscode.commands.executeCommand(
    'vscode.executeHoverProvider',
    docUri,
    pos2
  )) as vscode.Hover[];

  const contentarr2 = actualhover2[0].contents as vscode.MarkdownString[];

  assert.strictEqual(
    contentarr2[0].value,
    '```solidity\nevent SimpleAuction.HighestBidIncreased {\n\taddress bidder,\n\tuint256 amount\n}\n```'
  );

  const pos3 = new vscode.Position(53, 13);

  const actualhover3 = (await vscode.commands.executeCommand(
    'vscode.executeHoverProvider',
    docUri,
    pos3
  )) as vscode.Hover[];

  const contentarr3 = actualhover3[0].contents as vscode.MarkdownString[];

  assert.strictEqual(contentarr3[0].value, 'Abort execution if argument evaulates to false\n\n```solidity\n[built-in] void require (bool)\n```');
}

async function testdiagnos(docUri: vscode.Uri, expecteddiag: vscode.Diagnostic[]) {
  await activate(docUri);

  const actualDiagnostics = vscode.languages.getDiagnostics(docUri);

  if (actualDiagnostics) {
    expecteddiag.forEach((expectedDiagnostic, i) => {
      const actualDiagnostic = actualDiagnostics[i];
      assert.strictEqual(actualDiagnostic.message, expectedDiagnostic.message);
      assert.deepStrictEqual(actualDiagnostic.range, expectedDiagnostic.range);
      assert.strictEqual(actualDiagnostic.severity, expectedDiagnostic.severity);
    });
  } else {
    console.error('the diagnostics are incorrect', actualDiagnostics);
  }
}
