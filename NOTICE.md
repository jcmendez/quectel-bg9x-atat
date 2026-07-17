# Third-party attribution

The AT command, response, and type definitions in `src/commands/` were
originally adapted from [`quectel-bg9x-eh-driver`](https://gitlab.com/scrobotics/embedded-rs/quectel-atat-rs)
by SC Robotics, which targets the same Quectel BG95/BG96 AT command set via
the `atat` crate. That project is std-only and built on blocking
`embedded-hal`/`embedded-io` traits; this crate is a from-scratch `no_std`,
async (`embedded-io-async` via `atat`'s embassy integration) rewrite that
reuses its AT command struct definitions and field layouts as a starting
point, ported and adapted for this driver's async architecture.

Per the terms of that project's MIT license, its original copyright and
permission notice is reproduced below:

> The MIT License (MIT)
>
> Copyright (c) 2024 SC Robotics
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

The AT command semantics themselves (parameter names, ranges, meanings) come
from Quectel's public BG95&BG96 AT Commands Manual.
