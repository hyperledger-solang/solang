import { spawnSync } from 'child_process';

export default function executableVersion(path: string): string | undefined {
  console.debug('Checking availability of a binary at', path);
  const res = spawnSync(path, ['--version'], { encoding: 'utf8' });
  if (res.status == 0) {
    const match = res.stdout.match(/solang version v([\d.]+)/);
    if (match) {
      return match[1];
    }
  }
  return undefined;
}
