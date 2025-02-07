"use strict";

/**
 * Webpack configuration.
 */

exports = module.exports = {
  output: {
    library: "base32",
    libraryTarget: "this",
    sourcePrefix: ""
  },
  devtool: "source-map",
  node: {
    Buffer: false
  }
};
