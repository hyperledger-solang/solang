import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import _get from 'lodash/get';
import _keys from 'lodash/keys';
import _isUndefined from 'lodash/isUndefined';
import getManifest from '../../util/getManifest';

const MANIFEST_PROPERTIES_PATH :string = 'contributes.configuration.properties';

/**
 * @throws Error if the extension has no manifest, it cannot be read, or it
 *               contains invalid JSON.
 */
const getDefaultProperties = (context :vscode.ExtensionContext) :object => {
  const manifest :object = getManifest(context);
  const properties :object = _get(manifest, MANIFEST_PROPERTIES_PATH, {});
  const propertyNames :Array<string> = _keys(properties);
  const defaultProperties = {};

  propertyNames.forEach((name :string) :void => {
    const property :object = properties[name];
    const defaultValue = (<string | number>property.default);

    if (_isUndefined(defaultValue)) {
      return;
    }

    defaultProperties[name] = (<string | number>defaultValue);
  });

  return defaultProperties;
};

export default getDefaultProperties;
