# Contributors

[![Build Status][status-img]][status]

[status-img]: https://travis-ci.org/steveklabnik/contributors.svg?branch=master
[status]: https://travis-ci.org/steveklabnik/contributors

This web application shows people who have contributed to Rust.

It's very bad right now, sorry. But it technically works!

## Setup

You'll need nightly Rust for now. This will change with Rust 1.15, when
this will run on stable.

Get the app set up. You'll need postgres installed. And sqlite3 headers I
think.

Clone it:

```bash
$ git clone https://github.com/steveklabnik/contributors
$ cd contributors
```

Set up the database URL. Replace this with whatever credentials you need.

```bash
$ cp .env.sample .env
```

Inspect it to make sure it's set up the right way; only you can know what's
up with your local postgres install.

Build it:

```bash
$ cargo install diesel_cli
$ diesel setup
$ cargo build
```

Clone down the Rust repository somewhere. I put mine in `~/src`:

```bash
$ cd ~/src
$ git clone https://github.com/rust-lang/rust
```

Import data from the repo:

```bash
$ cd - # go back to our app
$ cargo run --bin populate -- --name Rust --path ~/src/rust --link https://github.com/rust-lang/rust # or whever you put the Rust source
```

This will take a few minutes. At the time of writing, Rust has about 61,000
commits that will need to be processed.

Run the server:

```bash
$ cargo run --bin contributors
```

Open your browser to the URL shown.

## Other stuff

To access the database from the commannd line:

```bash
psql -p 5432 -h localhost -U postgres -d rust_contributors
```

If you're working on the `populate` binary, it's useful to be able to quickly
drop your local database:

```bash
$ cargo run --bin the-big-red-button
```

When it's time for a new release,

```bash
$ cargo run --bin new-release -- --path ~/src/rust # or wherever your Rust is
```

As often as you want to update, run

```bash
$ cargo run --bin update-commit-db
```

This will hit GitHub's API instead of using a local checkout of Rust, as it is
assumed that this will run on the server, and we don't want to do a full git
checkout there.
