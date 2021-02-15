import * as vscode from 'vscode';
import * as path from 'path';
import * as crypto from 'crypto';
import { downloadFile } from './downloadFile';
import { promises as fs } from 'fs';

interface DownloadOptions {
  url: string;
  mode: number;
  dest: string;
  progressTitle: string;
}

export default async function download(opts: DownloadOptions) {
  const dest = path.parse(opts.dest);
  const randomHex = crypto.randomBytes(5).toString('hex');
  const tempFile = path.join(dest.dir, `${dest.name}${randomHex}`);

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      cancellable: false,
      title: opts.progressTitle,
    },
    async (progress) => {
      let lastPercentage = 0;
      await downloadFile(opts.url, tempFile, opts.mode, (readBytes, totalBytes) => {
        const newPercentage = Math.round((readBytes / totalBytes) * 100);
        if (newPercentage !== lastPercentage) {
          progress.report({
            message: `${newPercentage.toFixed(0)}%`,
            increment: newPercentage - lastPercentage,
          });

          lastPercentage = newPercentage;
        }
      });
    }
  );

  await fs.rename(tempFile, opts.dest);
}
