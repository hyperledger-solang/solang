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
        message: `unrecognised token 'aa', expected ";", "="`,
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
          `unrecognised token '}', expected "!", "(", "+", "++", "-", "--", "[", "address", "bool", "byte", "bytes", "case", "default", "delete", "error", "false", "function", "leave", "mapping", "new", "payable", "revert", "string", "switch", "this", "true", "type", "~", Bytes, Int, Uint, address, hexnumber, hexstring, identifier, number, rational, string`,
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
});

function toRange(lineno1: number, charno1: number, lineno2: number, charno2: number) {
  const start = new vscode.Position(lineno1, charno1);
  const end = new vscode.Position(lineno2, charno2);
  return new vscode.Range(start, end);
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

  assert.strictEqual(contentarr1[0].value, '(mapping(address => uint256))');

  const pos2 = new vscode.Position(78, 19);

  const actualhover2 = (await vscode.commands.executeCommand(
    'vscode.executeHoverProvider',
    docUri,
    pos2
  )) as vscode.Hover[];

  const contentarr2 = actualhover2[0].contents as vscode.MarkdownString[];

  assert.strictEqual(
    contentarr2[0].value,
    '```\nevent SimpleAuction.HighestBidIncreased {\n\taddress bidder,\n\tuint256 amount\n};\n```\n'
  );

  const pos3 = new vscode.Position(53, 13);

  const actualhover3 = (await vscode.commands.executeCommand(
    'vscode.executeHoverProvider',
    docUri,
    pos3
  )) as vscode.Hover[];

  const contentarr3 = actualhover3[0].contents as vscode.MarkdownString[];

  assert.strictEqual(contentarr3[0].value, '[built-in]  void require (bool): Abort execution if argument evaulates to false');
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
