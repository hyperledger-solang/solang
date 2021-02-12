import { OctoKit } from '@octokit/rest';
import { VERSION, GITHUB_API_TOKEN, LANGUAGE_SERVER_ID } from '../constants';

// TOOD: Consider expanding provided options
const client = new OctoKit({
  auth: GITHUB_API_TOKEN,
  userAgent: `${LANGUAGE_SERVER_ID} ${VERSION}`,
  log: console
});

export default client;
