import { assert } from 'console';
import * as vscode from 'vscode';
import * as path from 'path';
import { promises as fs } from 'fs';
import getPlatform from './getPlatform';
import downloadWithRetryDialog from './downloadWithRetryDialog';
import fetchLatestRelease from './fetchLatestRelease';
import download from './download';
import executableVersion from './executableVersion';
import { lte } from 'semver';

interface Artifact {
  name: string;
  browser_download_url: string;
}

export default async function getServer(context: vscode.ExtensionContext): Promise<string | undefined> {
  const config = vscode.workspace.getConfiguration('solang');

  const platfrom = getPlatform();
  if (platfrom === undefined) {
    await vscode.window.showErrorMessage("Unfortunately we don't ship binaries for your platform yet.");
    return undefined;
  }

  const dest = path.join(context.globalStoragePath, platfrom);
  const exists = await fs.stat(dest).then(
    () => true,
    () => false
  );
  if (!exists) {
    await context.globalState.update('serverVersion', undefined);
  }

  const release = await downloadWithRetryDialog(async () => {
    return await fetchLatestRelease();
  });
  console.log("Latest Solang available: " + release.tag_name);

  const latestVersion = release.tag_name;

  const ourVersion = executableVersion(dest);
  console.log("Local Solang version: " + ourVersion);
  if (ourVersion && lte(latestVersion, ourVersion)) {
    return dest;
  }

  if (config.get('updates.askBeforeDownload')) {
    const userResponse = await vscode.window.showInformationMessage(
      `Language server for solang ${latestVersion} is not installed.`,
      'Download now'
    );

    if (userResponse !== 'Download now') {
      return dest;
    }
  }

  const artifact = release.assets.find((artifact: Artifact) => artifact.name === platfrom);
  assert(!!artifact, `Bad release: ${JSON.stringify(release)}`);

  await downloadWithRetryDialog(async () => {
    await download({
      url: artifact.browser_download_url,
      dest,
      progressTitle: `Downloading Solang Solidity Compiler version ${latestVersion}`,
      mode: 0o755,
    });
  });

  return dest;
}
