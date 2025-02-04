import { XdrWriter } from '../../src/serialization/xdr-writer';
import { XdrReader } from '../../src/serialization/xdr-reader';
let Hyper = XDR.Hyper;

describe('Hyper.read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).to.eql(
      Hyper.fromString('0')
    );
    expect(read([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])).to.eql(
      Hyper.fromString('1')
    );
    expect(read([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])).to.eql(
      Hyper.fromString('-1')
    );
    expect(read([0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])).to.eql(
      new Hyper(Hyper.MAX_VALUE)
    );
    expect(read([0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).to.eql(
      new Hyper(Hyper.MIN_VALUE)
    );
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return Hyper.read(io);
  }
});

describe('Hyper.write', function () {
  it('encodes correctly', function () {
    expect(write(Hyper.fromString('0'))).to.eql([
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ]);
    expect(write(Hyper.fromString('1'))).to.eql([
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01
    ]);
    expect(write(Hyper.fromString('-1'))).to.eql([
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
    ]);
    expect(write(Hyper.MAX_VALUE)).to.eql([
      0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
    ]);
    expect(write(Hyper.MIN_VALUE)).to.eql([
      0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ]);
  });

  function write(value) {
    let io = new XdrWriter(8);
    Hyper.write(value, io);
    return io.toArray();
  }
});

describe('Hyper.isValid', function () {
  it('returns true for Hyper instances', function () {
    expect(Hyper.isValid(Hyper.MIN_VALUE)).to.be.true;
    expect(Hyper.isValid(Hyper.MAX_VALUE)).to.be.true;
    expect(Hyper.isValid(Hyper.fromString('0'))).to.be.true;
    expect(Hyper.isValid(Hyper.fromString('-1'))).to.be.true;
  });

  it('returns false for non Hypers', function () {
    expect(Hyper.isValid(null)).to.be.false;
    expect(Hyper.isValid(undefined)).to.be.false;
    expect(Hyper.isValid([])).to.be.false;
    expect(Hyper.isValid({})).to.be.false;
    expect(Hyper.isValid(1)).to.be.false;
    expect(Hyper.isValid(true)).to.be.false;
  });
});

describe('Hyper.fromString', function () {
  it('works for positive numbers', function () {
    expect(Hyper.fromString('1059').toString()).to.eql('1059');
  });

  it('works for negative numbers', function () {
    expect(Hyper.fromString('-1059').toString()).to.eql('-1059');
  });

  it('fails when providing a string with a decimal place', function () {
    expect(() => Hyper.fromString('105946095601.5')).to.throw(/bigint/);
  });
});
