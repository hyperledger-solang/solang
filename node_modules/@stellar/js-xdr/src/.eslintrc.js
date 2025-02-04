module.exports = {
  env: {
    es6: true,
    es2017: true,
    es2020: true,
    es2022: true
  },
  parserOptions: { ecmaVersion: 13 },
  extends: ['airbnb-base', 'prettier'],
  plugins: ['prettier', 'prefer-import'],
  rules: {
    // OFF
    'import/prefer-default-export': 0,
    'node/no-unsupported-features/es-syntax': 0,
    'node/no-unsupported-features/es-builtins': 0,
    camelcase: 0,
    'class-methods-use-this': 0,
    'linebreak-style': 0,
    'new-cap': 0,
    'no-param-reassign': 0,
    'no-underscore-dangle': 0,
    'no-use-before-define': 0,
    'prefer-destructuring': 0,
    'lines-between-class-members': 0,
    'no-plusplus': 0, // allow ++ for iterators
    'no-bitwise': 0, // allow high-performant bitwise operations

    // WARN
    'prefer-import/prefer-import-over-require': [1],
    'no-console': ['warn', { allow: ['assert'] }],
    'no-debugger': 1,
    'no-unused-vars': 1,
    'arrow-body-style': 1,
    'valid-jsdoc': [
      1,
      {
        requireReturnDescription: false
      }
    ],
    'prefer-const': 1,
    'object-shorthand': 1,
    'require-await': 1,
    'max-classes-per-file': ['warn', 3], // do not block imports from other classes

    // ERROR
    'no-unused-expressions': [2, { allowTaggedTemplates: true }],

    // we're redefining this without the Math.pow restriction
    // (since we don't want to manually add support for it)
    // copied from https://github.com/airbnb/javascript/blob/070e6200bb6c70fa31470ed7a6294f2497468b44/packages/eslint-config-airbnb-base/rules/best-practices.js#L200
    'no-restricted-properties': [
      'error',
      {
        object: 'arguments',
        property: 'callee',
        message: 'arguments.callee is deprecated'
      },
      {
        object: 'global',
        property: 'isFinite',
        message: 'Please use Number.isFinite instead'
      },
      {
        object: 'self',
        property: 'isFinite',
        message: 'Please use Number.isFinite instead'
      },
      {
        object: 'window',
        property: 'isFinite',
        message: 'Please use Number.isFinite instead'
      },
      {
        object: 'global',
        property: 'isNaN',
        message: 'Please use Number.isNaN instead'
      },
      {
        object: 'self',
        property: 'isNaN',
        message: 'Please use Number.isNaN instead'
      },
      {
        object: 'window',
        property: 'isNaN',
        message: 'Please use Number.isNaN instead'
      },
      {
        property: '__defineGetter__',
        message: 'Please use Object.defineProperty instead.'
      },
      {
        property: '__defineSetter__',
        message: 'Please use Object.defineProperty instead.'
      }
    ],
    'no-restricted-syntax': [
      // override basic rule to allow ForOfStatement
      'error',
      {
        selector: 'ForInStatement',
        message:
          'for..in loops iterate over the entire prototype chain, which is virtually never what you want. Use Object.{keys,values,entries}, and iterate over the resulting array.'
      },
      {
        selector: 'LabeledStatement',
        message:
          'Labels are a form of GOTO; using them makes code confusing and hard to maintain and understand.'
      },
      {
        selector: 'WithStatement',
        message:
          '`with` is disallowed in strict mode because it makes code impossible to predict and optimize.'
      }
    ]
  }
};
