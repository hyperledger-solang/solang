import * as XDR from '../../src';

describe('XDR.config', function () {
  beforeEach(function () {
    this.types = XDR.config(); // get the xdr object root
    for (const toDelete of Object.keys(this.types)) {
      delete this.types[toDelete];
    }
  });

  it('can define objects that have no dependency', function () {
    XDR.config((xdr) => {
      xdr.enum('Color', {
        red: 0,
        green: 1,
        blue: 2
      });

      xdr.enum('ResultType', {
        ok: 0,
        error: 1
      });
    }, this.types);

    expect(this.types.Color).to.exist;
    expect(this.types.ResultType).to.exist;
  });

  it('can define objects with the same name from different contexts', function () {
    XDR.config((xdr) => {
      xdr.enum('Color', {
        red: 0,
        green: 1,
        blue: 2
      });
    });

    XDR.config((xdr) => {
      xdr.enum('Color', {
        red: 0,
        green: 1,
        blue: 2
      });
    });
  });

  it('can define objects that have simple dependencies', function () {
    XDR.config((xdr) => {
      xdr.union('Result', {
        switchOn: xdr.lookup('ResultType'),
        switches: [
          ['ok', XDR.Void],
          ['error', 'message']
        ],
        defaultArm: XDR.Void,
        arms: {
          message: new XDR.String(100)
        }
      });

      xdr.enum('ResultType', {
        ok: 0,
        error: 1
      });
    }, this.types);

    expect(this.types.Result).to.exist;
    expect(this.types.ResultType).to.exist;

    let result = this.types.Result.ok();
    expect(result.switch()).to.eql(this.types.ResultType.ok());

    result = this.types.Result.error('It broke!');
    expect(result.switch()).to.eql(this.types.ResultType.error());
    expect(result.message()).to.eql('It broke!');
  });

  it('can define structs', function () {
    XDR.config((xdr) => {
      xdr.struct('Color', [
        ['red', xdr.int()],
        ['green', xdr.int()],
        ['blue', xdr.int()]
      ]);
    }, this.types);

    expect(this.types.Color).to.exist;

    let result = new this.types.Color({
      red: 0,
      green: 1,
      blue: 2
    });
    expect(result.red()).to.eql(0);
    expect(result.green()).to.eql(1);
    expect(result.blue()).to.eql(2);
  });

  it('can define typedefs', function () {
    let xdr = XDR.config((xdr) => {
      xdr.typedef('Uint256', xdr.opaque(32));
    });
    expect(xdr.Uint256).to.be.instanceof(XDR.Opaque);
  });

  it('can define consts', function () {
    let xdr = XDR.config((xdr) => {
      xdr.typedef('MAX_SIZE', 300);
    });
    expect(xdr.MAX_SIZE).to.eql(300);
  });

  it('can define arrays', function () {
    let xdr = XDR.config((xdr) => {
      xdr.typedef('ArrayOfInts', xdr.array(xdr.int(), 3));
      xdr.struct('MyStruct', [['red', xdr.int()]]);
      xdr.typedef('ArrayOfEmpty', xdr.array(xdr.lookup('MyStruct'), 5));
    });

    expect(xdr.ArrayOfInts).to.be.instanceof(XDR.Array);
    expect(xdr.ArrayOfInts._childType).to.eql(XDR.Int);
    expect(xdr.ArrayOfInts._length).to.eql(3);

    expect(xdr.ArrayOfEmpty).to.be.instanceof(XDR.Array);
    expect(xdr.ArrayOfEmpty._childType).to.eql(xdr.MyStruct);
    expect(xdr.ArrayOfEmpty._length).to.eql(5);
  });

  it('can define vararrays', function () {
    let xdr = XDR.config((xdr) => {
      xdr.typedef('ArrayOfInts', xdr.varArray(xdr.int(), 3));
    });

    expect(xdr.ArrayOfInts).to.be.instanceof(XDR.VarArray);
    expect(xdr.ArrayOfInts._childType).to.eql(XDR.Int);
    expect(xdr.ArrayOfInts._maxLength).to.eql(3);
  });

  it('can define options', function () {
    let xdr = XDR.config((xdr) => {
      xdr.typedef('OptionalInt', xdr.option(xdr.int()));
    });

    expect(xdr.OptionalInt).to.be.instanceof(XDR.Option);
    expect(xdr.OptionalInt._childType).to.eql(XDR.Int);
  });

  it('can use sizes defined as an xdr const', function () {
    let xdr = XDR.config((xdr) => {
      xdr.const('SIZE', 5);
      xdr.typedef('MyArray', xdr.array(xdr.int(), xdr.lookup('SIZE')));
      xdr.typedef('MyVarArray', xdr.varArray(xdr.int(), xdr.lookup('SIZE')));
      xdr.typedef('MyString', xdr.string(xdr.lookup('SIZE')));
      xdr.typedef('MyOpaque', xdr.opaque(xdr.lookup('SIZE')));
      xdr.typedef('MyVarOpaque', xdr.varOpaque(xdr.lookup('SIZE')));
    });

    expect(xdr.MyArray).to.be.instanceof(XDR.Array);
    expect(xdr.MyArray._childType).to.eql(XDR.Int);
    expect(xdr.MyArray._length).to.eql(5);

    expect(xdr.MyVarArray).to.be.instanceof(XDR.VarArray);
    expect(xdr.MyVarArray._childType).to.eql(XDR.Int);
    expect(xdr.MyVarArray._maxLength).to.eql(5);

    expect(xdr.MyString).to.be.instanceof(XDR.String);
    expect(xdr.MyString._maxLength).to.eql(5);

    expect(xdr.MyOpaque).to.be.instanceof(XDR.Opaque);
    expect(xdr.MyOpaque._length).to.eql(5);

    expect(xdr.MyVarOpaque).to.be.instanceof(XDR.VarOpaque);
    expect(xdr.MyVarOpaque._maxLength).to.eql(5);
  });
});
