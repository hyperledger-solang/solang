import { spawnSync } from 'child_process';

export default function isValidExecutable(path: string): boolean {
  console.debug('Checking availability of a binary at', path);
  const res = spawnSync(path, ['--version'], { encoding: 'utf8' });
  return res.status === 0;
}
