RUN = docker compose run --rm rust

## Unit tests
test:
	$(RUN) cargo test

## Static Linux binary: target/x86_64-unknown-linux-musl/release/wole-link
linux:
	$(RUN) cargo build --release --target x86_64-unknown-linux-musl

## Windows binary for the test VM: target/x86_64-pc-windows-gnu/release/wole-link.exe
windows:
	$(RUN) cargo build --release --target x86_64-pc-windows-gnu

.PHONY: test linux windows windows32 png

## 32-bit Windows binary, only for testing on a 32-bit VM:
## target/i686-pc-windows-gnu/release/wole-link.exe
windows32:
	$(RUN) cargo build --release --target i686-pc-windows-gnu

## The window in each language, as target/ui-*.png
png:
	$(RUN) cargo test -- --ignored png
