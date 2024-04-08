// SPDX-License-Identifier: Apache-2.0

import * as vscode from 'vscode';

export default async function downloadWithRetryDialog<T>(downloadFunc: () => Promise<T>): Promise<T> {
  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      return await downloadFunc();
    } catch (e: any) {
      const selected = await vscode.window.showErrorMessage(
        'Failed to download: ' + e.message,
        {},
        {
          title: 'Retry download',
          retry: true,
        },
        {
          title: 'Dismiss',
        }
      );

      if (selected?.retry) {
        continue;
      }
      throw e;
    }
  }
}
