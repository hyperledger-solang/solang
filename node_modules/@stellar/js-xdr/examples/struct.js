import * as XDR from '../src';

let xdr = XDR.config((xdr) => {
  xdr.struct('Signature', [
    ['publicKey', xdr.opaque(32)],
    ['data', xdr.opaque(32)]
  ]);

  xdr.struct('Envelope', [
    ['body', xdr.varOpaque(1000)],
    ['timestamp', xdr.uint()],
    ['signature', xdr.lookup('Signature')]
  ]);
});

let sig = new xdr.Signature();
sig.publicKey(Buffer.alloc(32));
sig.data(Buffer.from('00000000000000000000000000000000'));

let env = new xdr.Envelope({
  signature: sig,
  body: Buffer.from('hello'),
  timestamp: Math.floor(new Date() / 1000)
});

let output = env.toXDR();
let parsed = xdr.Envelope.fromXDR(output);

console.log(env);
console.log(parsed);
