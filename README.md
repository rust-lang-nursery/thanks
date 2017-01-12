# Contributors

This web application shows people who have contributed to Rust.

It's very bad right now, sorry. But it technically works!

## Setup

Get the app set up. You'll need postgres installed. And sqlite3 headers I
think.

Clone it:

```bash
$ git clone https://github.com/steveklabnik/contributors
$ cd contributors
```

Set up the database URL. Replace this with whatever credentials you need.

```bash
$ echo DATABASE_URL=postgres://postgres:postgres@localhost/rust_contributors > .env
```

Build it:

```bash
$ cargo build
$ cargo install diesel_cli
$ diesel setup
```

Clone down the Rust repository somewhere. I put mine in `~/src`:

```bash
$ cd ~/src
$ git clone https://github.com/rust-lang/rust
```

Import data from the repo:

```bash
$ cd - # go back to our app
$ cargo run --bin populate -- ~/src/rust # or whever you put the Rust source
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
$ cargo run --bin new-release
```
