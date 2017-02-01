# Thanks!

[![Build Status][status-img]][status]

[status-img]: https://travis-ci.org/rust-lang-nursery/thanks.svg?branch=master
[status]: https://travis-ci.org/rust-lang-nursery/thanks

This web application shows people who have contributed to Rust.

## Setup

You'll need nightly Rust for now. This will change with Rust 1.15, when
this will run on stable.

Get the app set up. You'll need [postgres](diesel_setup.md) installed. And
[sqlite3 headers](diesel_setup.md) I think.

Clone it:

```bash
$ git clone https://github.com/rust-lang-nursery/thanks
$ cd thanks
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
$ cargo run --bin populate -- \
    --name Rust \
    --github rust-lang/rust \
    --url https://github.com/rust-lang/rust/ \
    --path ~/src/rust # or wherever you put the Rust source
```

This will take a few minutes. At the time of writing, Rust has about 61,000
commits that will need to be processed.

Run the server:

```bash
$ cargo run --bin thanks
```

Open your browser to the URL shown.

## Other stuff

To access the database from the commannd line:

```bash
psql -p 5432 -h localhost -U postgres -d thanks
```

If you have the database with the old name (`rust_contributors` or any
other), you have two options:
- use the old name in the above command, or:
- run `psql -p 5432 -h localhost -U postgres`, rename the database by running
  `ALTER DATABASE rust_contributors RENAME TO thanks` and edit `.env` file to
  use the new name.

If you're working on the `populate` binary, it's useful to be able to quickly
drop your local database:

```bash
$ cargo run --bin the-big-red-button --all
```

You can also delete only one project by passing `--name NAME` option.

When it's time for a new release,

```bash
$ cargo run --bin new-release -- --name Rust --version 1.15.0 --path ~/src/rust # or wherever your Rust is
```

As often as you want to update, run

```bash
$ cargo run --bin update-commit-db
```

This will hit GitHub's API instead of using a local checkout of Rust, as it is
assumed that this will run on the server, and we don't want to do a full git
checkout there.
