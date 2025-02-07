"use strict";

/**
 * Karma configuration.
 */

module.exports = function (config) {
  config.set({

    frameworks: ["mocha"],

    files: [
      "test/**_test.js"
    ],

    preprocessors: {
      "test/**_test.js": ["webpack"]
    },

    reporters: ["progress"],

    browsers: ["Chrome"],

    webpack: require("./webpack.config"),

    plugins: [
      "karma-chrome-launcher",
      "karma-mocha",
      "karma-webpack"
    ]

  });
};
