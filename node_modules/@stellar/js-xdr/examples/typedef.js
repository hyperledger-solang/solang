import * as XDR from '../src';

let xdr = XDR.config((xdr) => {
  xdr.struct('Signature', [
    ['publicKey', xdr.opaque(32)],
    ['data', xdr.opaque(32)]
  ]);

  xdr.typedef('SignatureTypedef', xdr.lookup('Signature'));
  xdr.typedef('IntTypedef', xdr.int());
});

console.log(xdr.SignatureTypedef === xdr.Signature);
console.log(xdr.IntTypedef === XDR.Int);
