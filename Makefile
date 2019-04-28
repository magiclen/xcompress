all: ./target/x86_64-unknown-linux-musl/release/xcompress

./target/x86_64-unknown-linux-musl/release/xcompress: $(shell find . -type f -iname '*.rs' -o -name 'Cargo.toml' | sed 's/ /\\ /g')
	cargo build --release --target x86_64-unknown-linux-musl
	strip ./target/x86_64-unknown-linux-musl/release/xcompress
	
install:
	$(MAKE)
	sudo cp ./target/x86_64-unknown-linux-musl/release/xcompress /usr/local/bin/xcompress
	sudo chown root. /usr/local/bin/xcompress
	sudo chmod 0755 /usr/local/bin/xcompress

uninstall:
	sudo rm /usr/local/bin/xcompress

test:
	cargo test --verbose

clean:
	cargo clean
