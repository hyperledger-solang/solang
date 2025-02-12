module.exports = {
  env: {
    node: true
  },
  extends: ['eslint:recommended', 'plugin:node/recommended'],
  rules: {
    'node/no-unpublished-require': 0,
    indent: ['warn', 2]
  }
};
