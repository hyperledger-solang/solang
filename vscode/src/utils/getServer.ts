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

  const platform = getPlatform();
  if (platform === undefined) {
    await vscode.window.showErrorMessage("Unfortunately we don't ship binaries for your platform yet.");
    return undefined;
  }

  const dest = path.join(context.globalStoragePath, platform);
  const exists = await fs.stat(dest).then(
    () => true,
    () => false
  );
  if (!exists) {
    await context.globalState.update('serverVersion', undefined);
  }

  const ourVersion = executableVersion(dest);
  console.log("Local Solang version: " + ourVersion);

  let release;

  try {
    release = await fetchLatestRelease();
  }
  catch (e) {
    if (e instanceof Error && ourVersion !== undefined) {
      // we failed to get the latest release version, but we do have a local copy
      console.log("Failed to download: " + e.message)
      return dest;
    }
    throw (e);
  }

  console.log("Latest Solang available: " + release.tag_name);

  const latestVersion = release.tag_name;

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

  const artifact = release.assets.find((artifact: Artifact) => artifact.name === platform);

  if (artifact === undefined) {
    console.error(`cannot find download for ${platform}`);
  } else {
    await downloadWithRetryDialog(async () => {
      await download({
        url: artifact.browser_download_url,
        dest,
        progressTitle: `Downloading Solang Solidity Compiler version ${latestVersion}`,
        mode: 0o755,
      });
    });
  }

  return dest;
}
