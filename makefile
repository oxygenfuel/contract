build:
	forc build
test:
	cargo test -- --nocapture
deploy:
	cargo test --package orderbook --test integration_tests -- deploy --exact --show-output
clean:
	 