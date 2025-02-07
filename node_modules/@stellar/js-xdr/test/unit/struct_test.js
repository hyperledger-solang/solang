import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';
import { Struct } from '../../src/struct';

/* jshint -W030 */

let emptyContext = { definitions: {}, results: {} };
let MyRange = XDR.Struct.create(emptyContext, 'MyRange', [
  ['begin', XDR.Int],
  ['end', XDR.Int],
  ['inclusive', XDR.Bool]
]);

describe('Struct.read', function () {
  it('decodes correctly', function () {
    let empty = read([
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ]);
    expect(empty).to.be.instanceof(MyRange);
    expect(empty.begin()).to.eql(0);
    expect(empty.end()).to.eql(0);
    expect(empty.inclusive()).to.eql(false);

    let filled = read([
      0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x01
    ]);
    expect(filled).to.be.instanceof(MyRange);
    expect(filled.begin()).to.eql(5);
    expect(filled.end()).to.eql(255);
    expect(filled.inclusive()).to.eql(true);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return MyRange.read(io);
  }
});

describe('Struct.write', function () {
  it('encodes correctly', function () {
    let empty = new MyRange({
      begin: 0,
      end: 0,
      inclusive: false
    });

    expect(write(empty)).to.eql([
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ]);

    let filled = new MyRange({
      begin: 5,
      end: 255,
      inclusive: true
    });

    expect(write(filled)).to.eql([
      0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x01
    ]);
  });

  it('throws a write error if the value is not the correct type', function () {
    expect(() => write(null)).to.throw(/write error/i);
    expect(() => write(undefined)).to.throw(/write error/i);
    expect(() => write([])).to.throw(/write error/i);
    expect(() => write({})).to.throw(/write error/i);
    expect(() => write(1)).to.throw(/write error/i);
    expect(() => write(true)).to.throw(/write error/i);
  });

  it('throws a write error if the struct is not valid', function () {
    expect(() => write(new MyRange({}))).to.throw(/write error/i);
  });

  function write(value) {
    let io = new XdrWriter(256);
    MyRange.write(value, io);
    return io.toArray();
  }
});

describe('Struct.isValid', function () {
  it('returns true for instances of the struct', function () {
    expect(MyRange.isValid(new MyRange({}))).to.be.true;
  });

  it('works for "struct-like" objects', function () {
    class FakeStruct extends Struct {}

    FakeStruct.structName = 'MyRange';
    let r = new FakeStruct();
    expect(MyRange.isValid(r)).to.be.true;

    FakeStruct.structName = 'NotMyRange';
    r = new FakeStruct();
    expect(MyRange.isValid(r)).to.be.false;
  });

  it('returns false for anything else', function () {
    expect(MyRange.isValid(null)).to.be.false;
    expect(MyRange.isValid(undefined)).to.be.false;
    expect(MyRange.isValid([])).to.be.false;
    expect(MyRange.isValid({})).to.be.false;
    expect(MyRange.isValid(1)).to.be.false;
    expect(MyRange.isValid(true)).to.be.false;
  });
});

describe('Struct.validateXDR', function () {
  it('returns true for valid XDRs', function () {
    let subject = new MyRange({ begin: 5, end: 255, inclusive: true });
    expect(MyRange.validateXDR(subject.toXDR())).to.be.true;
    expect(MyRange.validateXDR(subject.toXDR('hex'), 'hex')).to.be.true;
    expect(MyRange.validateXDR(subject.toXDR('base64'), 'base64')).to.be.true;
  });

  it('returns false for invalid XDRs', function () {
    expect(MyRange.validateXDR(Buffer.alloc(1))).to.be.false;
    expect(MyRange.validateXDR('00', 'hex')).to.be.false;
    expect(MyRange.validateXDR('AA==', 'base64')).to.be.false;
  });
});

describe('Struct: attributes', function () {
  it('properly retrieves attributes', function () {
    let subject = new MyRange({ begin: 5, end: 255, inclusive: true });
    expect(subject.begin()).to.eql(5);
  });

  it('properly sets attributes', function () {
    let subject = new MyRange({ begin: 5, end: 255, inclusive: true });
    expect(subject.begin(10)).to.eql(10);
    expect(subject.begin()).to.eql(10);
  });
});
