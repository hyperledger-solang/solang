if (typeof global === 'undefined') {
  // eslint-disable-next-line no-undef
  window.global = window;
}
global['XDR'] = require('../src');
global.chai = require('chai');
global.sinon = require('sinon');
global.chai.use(require('sinon-chai'));

global.expect = global.chai.expect;

exports.mochaHooks = {
  beforeEach: function () {
    this.sandbox = global.sinon.createSandbox();
    global.stub = this.sandbox.stub.bind(this.sandbox);
    global.spy = this.sandbox.spy.bind(this.sandbox);
  },
  afterEach: function () {
    delete global.stub;
    delete global.spy;
    this.sandbox.restore();
  }
};
