import * as path from 'path';
import { homedir } from 'os';

// TODO: Doc
const expandPathTilde = (path: string) :string => (
  path.startsWith('~/')
    ? path.replace('~', homedir())
    : path
);

export default expandPathTilde;
