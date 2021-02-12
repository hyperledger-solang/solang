import { homedir } from 'os';
import * as path from 'path';

// TODO: Refactor rest of extension to define this as a constant
const LANGUAGE_SERVER_ID :string = 'solang';
const LANGUAGE_SERVER_NAME :string = 'Solang Solidity Compiler';
const GITHUB_OWNER :string = 'hyperledger-labs';
const GITHUB_REPO :string = 'solang';

// TODO: Get this from elsewhere; manifest is out of rootDir
const VERSION :string = '0.0.2';

// const { configuration :object } : { configuration } = contributes;
// const { properties :object } : { properties } = configuration;

// TODO: Switch to signale if output is rendered properly
// TODO: Token prob not needed, refactor
const GITHUB_API_TOKEN :string = '';
const DEFAULT_SOLANG_BIN_PATH :string = path.join(homedir(), '.cargo/bin/solang');

const CONFIG_KEY_COMMAND :string = 'languageServerExecutable';
const CONFIG_KEY_TARGET :string = 'target';

export {
  CONFIG_KEY_TARGET,
  CONFIG_KEY_COMMAND,
  DEFAULT_SOLANG_BIN_PATH,
  LANGUAGE_SERVER_NAME,
  LANGUAGE_SERVER_ID,
  GITHUB_API_TOKEN,
  GITHUB_OWNER,
  GITHUB_REPO,
  VERSION
};
