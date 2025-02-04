const webpack = require('webpack');

module.exports = function (config) {
  config.set({
    frameworks: ['mocha', 'webpack', 'sinon-chai'],
    browsers: ['FirefoxHeadless', 'ChromeHeadless'],
    browserNoActivityTimeout: 20000,

    files: ['dist/xdr.js', 'test/unit/**/*.js'],

    preprocessors: {
      'test/unit/**/*.js': ['webpack']
    },

    webpack: {
      mode: 'development',
      module: {
        rules: [
          { test: /\.js$/, exclude: /node_modules/, loader: 'babel-loader' }
        ]
      },
      plugins: [
        new webpack.ProvidePlugin({
          Buffer: ['buffer', 'Buffer']
        })
      ]
    },

    webpackMiddleware: {
      noInfo: true
    },

    singleRun: true,

    reporters: ['dots']
  });
};
