// SPDX-License-Identifier: Apache-2.0

import { homedir } from 'os';

export default function expandPathResolving(path: string): string {
  if (path.startsWith('~/')) {
    return path.replace('~', homedir());
  }
  return path;
}
