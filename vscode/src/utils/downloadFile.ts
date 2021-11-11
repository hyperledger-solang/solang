import fetch from 'node-fetch';
import { PathLike, createWriteStream } from 'fs';
import * as stream from 'stream';
import * as util from 'util';
import { assert } from 'console';

const pipeline = util.promisify(stream.pipeline);

export async function downloadFile(
  url: string,
  destFilePath: PathLike,
  mode: number | undefined,
  onProgress: (readBytes: number, totalBytes: number) => void
) {
  const res = await fetch(url);

  if (!res.ok) {
    console.error('Error', res.status, 'while downloading file from', url);
    console.error({ body: await res.text(), headers: res.headers });

    throw new Error(`Got response ${res.status} when trying to download a file.`);
  }

  const totalBytes = Number(res.headers.get('content-length'));
  assert(!Number.isNaN(totalBytes), 'Sanity check of content-length protocol');

  console.debug('Downloading file of', totalBytes, 'bytes size from', url, 'to', destFilePath);

  let readBytes = 0;
  res.body.on('data', (chunk: Buffer) => {
    readBytes += chunk.length;
    onProgress(readBytes, totalBytes);
  });

  const destFileStream = createWriteStream(destFilePath, { mode });
  const srcStream = res.body;

  await pipeline(srcStream, destFileStream);

  // Don't apply the workaround in fixed versions of nodejs, since the process
  // freezes on them, the process waits for no-longer emitted `close` event.
  // The fix was applied in commit 7eed9d6bcc in v13.11.0
  // See the nodejs changelog:
  // https://github.com/nodejs/node/blob/master/doc/changelogs/CHANGELOG_V13.md
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const [, major, minor] = /v(\d+)\.(\d+)\.(\d+)/.exec(process.version)!;
  if (+major > 13 || (+major === 13 && +minor >= 11)) {
    return;
  }

  await new Promise<void>((resolve) => {
    destFileStream.on('close', resolve);
    destFileStream.destroy();
    // This workaround is awaiting to be removed when vscode moves to newer nodejs version:
  });
}
