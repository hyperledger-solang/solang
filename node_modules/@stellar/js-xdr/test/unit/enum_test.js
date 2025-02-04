import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';
import { Enum } from '../../src/enum';

/* jshint -W030 */

let emptyContext = { definitions: {}, results: {} };
let Color = XDR.Enum.create(emptyContext, 'Color', {
  red: 0,
  green: 1,
  evenMoreGreen: 3
});

describe('Enum.fromName', function () {
  it('returns the member with the provided name', function () {
    expect(Color.fromName('red')).to.eql(Color.red());
    expect(Color.fromName('green')).to.eql(Color.green());
    expect(Color.fromName('evenMoreGreen')).to.eql(Color.evenMoreGreen());
  });

  it('throws an error if the name is not correct', function () {
    expect(() => Color.fromName('obviouslyNotAColor')).to.throw(
      /not a member/i
    );
  });
});

describe('Enum.fromValue', function () {
  it('returns the member with the provided value', function () {
    expect(Color.fromValue(0)).to.eql(Color.red());
    expect(Color.fromValue(1)).to.eql(Color.green());
    expect(Color.fromValue(3)).to.eql(Color.evenMoreGreen());
  });

  it('throws an error if the value is not correct', function () {
    expect(() => Color.fromValue(999)).to.throw(/not a value/i);
  });
});

describe('Enum.read', function () {
  it('decodes correctly', function () {
    expect(read([0x00, 0x00, 0x00, 0x00])).to.eql(Color.red());
    expect(read([0x00, 0x00, 0x00, 0x01])).to.eql(Color.green());
    expect(read([0x00, 0x00, 0x00, 0x03])).to.eql(Color.evenMoreGreen());
  });

  it("throws read error when encoded value isn't defined on the enum", function () {
    expect(() => read([0x00, 0x00, 0x00, 0x02])).to.throw(/read error/i);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return Color.read(io);
  }
});

describe('Enum.write', function () {
  it('encodes correctly', function () {
    expect(write(Color.red())).to.eql([0x00, 0x00, 0x00, 0x00]);
    expect(write(Color.green())).to.eql([0x00, 0x00, 0x00, 0x01]);
    expect(write(Color.evenMoreGreen())).to.eql([0x00, 0x00, 0x00, 0x03]);

    expect(Color.red().toXDR('hex')).to.eql('00000000');
    expect(Color.green().toXDR('hex')).to.eql('00000001');
    expect(Color.evenMoreGreen().toXDR('hex')).to.eql('00000003');
  });

  it('throws a write error if the value is not the correct type', function () {
    expect(() => write(null)).to.throw(/write error/i);
    expect(() => write(undefined)).to.throw(/write error/i);
    expect(() => write([])).to.throw(/write error/i);
    expect(() => write({})).to.throw(/write error/i);
    expect(() => write(1)).to.throw(/write error/i);
    expect(() => write(true)).to.throw(/write error/i);
  });

  function write(value) {
    let io = new XdrWriter(8);
    Color.write(value, io);
    return io.toArray();
  }
});

describe('Enum.isValid', function () {
  it('returns true for members of the enum', function () {
    expect(Color.isValid(Color.red())).to.be.true;
    expect(Color.isValid(Color.green())).to.be.true;
    expect(Color.isValid(Color.evenMoreGreen())).to.be.true;
  });

  it('works for "enum-like" objects', function () {
    class FakeEnum extends Enum {}
    FakeEnum.enumName = 'Color';

    let r = new FakeEnum();
    expect(Color.isValid(r)).to.be.true;

    FakeEnum.enumName = 'NotColor';
    r = new FakeEnum();
    expect(Color.isValid(r)).to.be.false;

    // make sure you can't fool it
    FakeEnum.enumName = undefined;
    FakeEnum.unionName = 'Color';
    r = new FakeEnum();
    expect(Color.isValid(r)).to.be.false;
  });

  it('returns false for arrays of the wrong size', function () {
    expect(Color.isValid(null)).to.be.false;
    expect(Color.isValid(undefined)).to.be.false;
    expect(Color.isValid([])).to.be.false;
    expect(Color.isValid({})).to.be.false;
    expect(Color.isValid(1)).to.be.false;
    expect(Color.isValid(true)).to.be.false;
  });
});
