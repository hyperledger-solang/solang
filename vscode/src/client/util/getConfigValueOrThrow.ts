import * as vscode from 'vscode';
import { window, workspace } from 'vscode';
import _isUndefined from 'lodash/isUndefined';

import getDefaultProperties from './getDefaultProperties';
import { LANGUAGE_SERVER_ID } from '../constants';

// TODO: Refactor to grab extension/config from context
const getConfigValueOrThrow = async (
  name :string,
  context :vscode.ExtensionContext
) :Promise<string> => {
  const defaultProperties :object = getDefaultProperties(context);
  const defaultValue :any = defaultProperties[name];
	const config :vscode.WorkspaceConfiguration = workspace.getConfiguration(LANGUAGE_SERVER_ID);
	const value :string | undefined = config.get(name, defaultValue);

  if (_isUndefined(value)) {
    const errMessage :string = `Config setting ${name} is not defined and required`;

    window.showErrorMessage(errMessage);
    throw new Error(errMessage);
  }

  return value;
};

export default getConfigValueOrThrow;
