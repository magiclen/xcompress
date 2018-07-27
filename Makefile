all: ./target/release/xcompress

./target/release/xcompress: ./src/lib.rs ./src/main.rs
	cargo build --release
	strip ./target/release/xcompress
	
install:
	$(MAKE)
	sudo cp ./target/release/xcompress /usr/local/bin/xcompress
	sudo chown root. /usr/local/bin/xcompress
	sudo chmod 0755 /usr/local/bin/xcompress
	
clean:
	cargo clean
