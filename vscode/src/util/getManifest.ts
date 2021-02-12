import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

/**
 * @throws Error if the extension has no manifest, it cannot be read, or it
 *               contains invalid JSON.
 */
const getManifest = (context :vscode.ExtensionContext) :object => {
  const { extensionPath :any } : { extensionPath } = context;
  const manifestPath :string = path.join((<string>extensionPath), 'package.json');
  let manifestJSON :string;

  if (!fs.existsSync(manifestPath)) {
    throw new Error(`No manifest found in extension path: ${extensionPath}`);
  }

  try {
    manifestJSON = fs.readFileSync(manifestPath, 'utf-8');
  } catch (e) {
    throw new Error('Error reading extension manifest ${manifestPath}: ${e.message}');
  }

  try {
    return JSON.parse(manifestJSON);
  } catch (e) {
    throw new Error('Error parsing extension manifest ${manifestPath}: ${e.message}');
  }
};

export default getManifest;
