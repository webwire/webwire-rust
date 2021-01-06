all:
	(cd ../../../webwire-cli && cargo run gen rust ) < ../chat.ww | rustfmt --edition 2018 > api/src/lib.rs
	cargo build

api/src/lib.rs: ../chat.ww
	(cd ../../../webwire-cli && cargo run gen rust ) < ../chat.ww | rustfmt --edition 2018 > api/src/lib.rs

run:
	cargo run
