coverage:
	cargo llvm-cov clean --workspace
	cargo llvm-cov --workspace --html
	@echo "HTML coverage at target/llvm-cov/html/index.html"

coverage-lcov:
	mkdir -p coverage
	cargo llvm-cov clean --workspace
	cargo llvm-cov --workspace --lcov --output-path coverage/lcov.info
	@echo "LCOV written to coverage/lcov.info"

coverage-clean:
	cargo llvm-cov clean --workspace

