# Base 32 for JavaScript [![Build Status](https://travis-ci.org/mikepb/base32.js.svg)](http://travis-ci.org/mikepb/base32.js)

[Wikipedia](https://en.wikipedia.org/wiki/Base32):

> Base32 is a base 32 transfer encoding using the twenty-six letters A–Z and six digits 2–7. It is primarily used to encode binary data, but is able to encode binary text like ASCII.
>
> Base32 has number of advantages over Base64:
>
> 1. The resulting character set is all one case (usually represented as uppercase), which can often be beneficial when using a case-insensitive filesystem, spoken language, or human memory.
>
> 2. The result can be used as file name because it can not possibly contain '/' symbol which is usually acts as path separator in Unix-based operating systems.
>
> 3. The alphabet was selected to avoid similar-looking pairs of different symbols, so the strings can be accurately transcribed by hand. (For example, the symbol set omits the symbols for 1, 8 and zero, since they could be confused with the letters 'I', 'B', and 'O'.)
>
> 4. A result without padding can be included in a URL without encoding any characters.
>
> However, Base32 representation takes roughly 20% more space than Base64.

## Documentation

Full documentation at http://mikepb.github.io/base32.js/

## Installation

```sh
$ npm install base32.js
```

## Usage

Encoding an array of bytes using [Crockford][crock32]:

```js
var base32 = require("base32.js");

var buf = [1, 2, 3, 4];
var encoder = new base32.Encoder({ type: "crockford", lc: true });
var str = encoder.write(buf).finalize();
// str = "04106173"

var decoder = new base32.Decoder({ type: "crockford" });
var out = decoder.write(str).finalize();
// out = [1, 2, 3, 4]
```

The default Base32 variant if no `type` is provided is `"rfc4648"` without
padding.

## Browser support

The browser versions of the library may be found under the `dist/` directory.
The browser files are updated on each versioned release, but not for
development. [Karma][karma] is used to run the [mocha][] tests in the browser.

```sh
$ npm install -g karma-cli
$ npm run karma
```

## Related projects

- [agnoster/base32-js][agnoster]

## License

MIT

[agnoster]: https://github.com/agnoster/base32-js
[crock32]: http://www.crockford.com/wrmg/base32.html
[karma]: http://karma-runner.github.io
[mocha]: http://mochajs.org
