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

To import some test data:

```bash
psql -p 5432 -h localhost -U postgres < dump.txt
```

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
