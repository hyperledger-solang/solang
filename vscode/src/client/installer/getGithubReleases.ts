import octokitClient from './octokit';
import { GITHUB_OWNER, GITHUB_REPO } from '../constants';

const getGithubReleases = async () :Promise<Array<Object>> => (
  octokitClient.repos.listReleases({
    owner: GITHUB_OWNER,
    repo: GITHUB_REPO
  })
);

export default getGithubReleases;
