import { XdrReader } from '../../src/serialization/xdr-reader';
import { XdrWriter } from '../../src/serialization/xdr-writer';
import { XdrPrimitiveType } from '../../src/xdr-type';

/* jshint -W030 */

let emptyContext = { definitions: {}, results: {} };
let ResultType = XDR.Enum.create(emptyContext, 'ResultType', {
  ok: 0,
  error: 1,
  nonsense: 2
});

let Result = XDR.Union.create(emptyContext, 'Result', {
  switchOn: ResultType,
  switches: [
    ['ok', XDR.Void],
    ['error', 'code']
  ],
  defaultArm: XDR.Void,
  arms: {
    code: XDR.Int
  }
});

let Ext = XDR.Union.create(emptyContext, 'Ext', {
  switchOn: XDR.Int,
  switches: [[0, XDR.Void]]
});

describe('Union.armForSwitch', function () {
  it('returns the defined arm for the provided switch', function () {
    expect(Result.armForSwitch(ResultType.ok())).to.eql(XDR.Void);
    expect(Result.armForSwitch(ResultType.error())).to.eql('code');
  });

  it('returns the default arm if no specific arm is defined', function () {
    expect(Result.armForSwitch(ResultType.nonsense())).to.eql(XDR.Void);
  });

  it('works for XDR.Int discriminated unions', function () {
    expect(Ext.armForSwitch(0)).to.eql(XDR.Void);
  });
});

describe('Union: constructor', function () {
  it('works for XDR.Int discriminated unions', function () {
    expect(() => new Ext(0)).to.not.throw();
  });

  it('works for Enum discriminated unions', function () {
    expect(() => new Result('ok')).to.not.throw();
    expect(() => new Result(ResultType.ok())).to.not.throw();
  });
});

describe('Union: set', function () {
  it('works for XDR.Int discriminated unions', function () {
    let u = new Ext(0);
    u.set(0);
  });

  it('works for Enum discriminated unions', function () {
    let u = Result.ok();

    expect(() => u.set('ok')).to.not.throw();
    expect(() => u.set('notok')).to.throw(/not a member/);
    expect(() => u.set(ResultType.ok())).to.not.throw();
  });
});

describe('Union.read', function () {
  it('decodes correctly', function () {
    let ok = read([0x00, 0x00, 0x00, 0x00]);

    expect(ok).to.be.instanceof(Result);
    expect(ok.switch()).to.eql(ResultType.ok());
    expect(ok.arm()).to.eql(XDR.Void);
    expect(ok.armType()).to.eql(XDR.Void);
    expect(ok.value()).to.be.undefined;

    let error = read([0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x05]);

    expect(error).to.be.instanceof(Result);
    expect(error.switch()).to.eql(ResultType.error());
    expect(error.arm()).to.eql('code');
    expect(error.armType()).to.eql(XDR.Int);
    expect(error.value()).to.eql(5);
    expect(error.code()).to.eql(5);
  });

  function read(bytes) {
    let io = new XdrReader(bytes);
    return Result.read(io);
  }
});

describe('Union.write', function () {
  it('encodes correctly', function () {
    let ok = Result.ok();

    expect(write(ok)).to.eql([0x00, 0x00, 0x00, 0x00]);

    let error = Result.error(5);

    expect(write(error)).to.eql([
      0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x05
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

  function write(value) {
    let io = new XdrWriter(256);
    Result.write(value, io);
    return io.toArray();
  }
});

describe('Union.isValid', function () {
  it('returns true for instances of the union', function () {
    expect(Result.isValid(Result.ok())).to.be.true;
    expect(Result.isValid(Result.error(1))).to.be.true;
    expect(Result.isValid(Result.nonsense())).to.be.true;
  });

  it('works for "union-like" objects', function () {
    class FakeUnion extends XdrPrimitiveType {}

    FakeUnion.unionName = 'Result';
    let r = new FakeUnion();
    expect(Result.isValid(r)).to.be.true;

    FakeUnion.unionName = 'NotResult';
    r = new FakeUnion();
    expect(Result.isValid(r)).to.be.false;

    // make sure you can't fool it
    FakeUnion.unionName = undefined;
    FakeUnion.structName = 'Result';
    r = new FakeUnion();
    expect(Result.isValid(r)).to.be.false;
  });

  it('returns false for anything else', function () {
    expect(Result.isValid(null)).to.be.false;
    expect(Result.isValid(undefined)).to.be.false;
    expect(Result.isValid([])).to.be.false;
    expect(Result.isValid({})).to.be.false;
    expect(Result.isValid(1)).to.be.false;
    expect(Result.isValid(true)).to.be.false;
    expect(Result.isValid('ok')).to.be.false;
  });
});
