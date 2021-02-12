import Bluebird from 'bluebird';

/**
 * Called when extension is deactivated, optionally cleans up downloaded solang
 * binaries.
 *
 * @TODO implement
 */
const deactivate = async () :Promise<void> => Bluebird.resolve();

export default deactivate;
