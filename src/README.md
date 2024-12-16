# ansi-strip

Used for stripping ANSI escape codes from text. This does not contain a complete implementation of an ANSI escape code parser, it just does what I need it to do: remove common formatting codes.

It provides `AnsiStripper` which does the work, and a trait called `NonEsc` which is implemented for `&str` so that one can just call `.non_esc()` against any string slice to do the business. There's also an installable binary, `ansi-strip`, that reads from stdin and forwards the stripped strings to stdout.

