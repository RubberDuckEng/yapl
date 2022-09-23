# yapl
 Yet Another Programing Language

Transforms JSON to JSON using YAML as syntax.

# How to run

```sh
$ cargo run --bin interp examples/hello_world.yapl
```

# TODO
* Fix + to be on the function key, not the non-function keys.
* Finish making hello_servlet.yapl work.

# Things to try
* Immutable?
* No classes

# Programs to write
* Some simple adventure game
* JSON RPC server

# Library to write
* HTML serializer / JSON to HTML (e.g. @HTML)
* JSON scheme validator
* Library for string manipulation
* Library for doing math

# Similar projects
* [https://github.com/scravy/jinsi]
* [https://github.com/benlue/jsonfp]
* [https://stackoverflow.com/questions/1618038/xslt-equivalent-for-json]

# Constructs that should be expressible

* List and object comprehensions
  - `[for (<expr>) <expr>]`
  - `{for (<expr>) <expr> : <expr>}`
* Source of stdlib ideas
  - [https://github.com/chrisdone/jl#available-functions]
* Pathing syntax
  - [https://goessner.net/articles/JsonPath/]
  - [https://github.com/dfilatov/jspath]
