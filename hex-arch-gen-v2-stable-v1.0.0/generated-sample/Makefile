.PHONY: build run check test fmt lint migrate clean sbom lock

build:
        cargo build --release

run:
        cargo run

check:
        cargo check --all-targets

test:
        cargo test

fmt:
        cargo fmt --all

migrate:
        @echo "Apply migrations: psql $$DATABASE_URL -f migrations/*.sql"
        @if [ -z "$$DATABASE_URL" ]; then echo "DATABASE_URL not set"; exit 1; fi
        for f in migrations/*.sql; do psql "$$DATABASE_URL" -f "$$f"; done

clean:
        cargo clean

# v0.4.0 (closes G-H10): CycloneDX SBOM in JSON format.
# Requires `cargo install cargo-cyclonedx` once.
sbom:
        @command -v cargo-cyclonedx >/dev/null 2>&1 || { \
                echo "cargo-cyclonedx not installed — run: cargo install cargo-cyclonedx"; \
                exit 1; \
        }
        mkdir -p sbom
        cargo cyclonedx -f json --output-directory sbom/
        @echo "SBOM written to sbom/*.json"

# v0.5.0 (closes G-L04): generate / refresh Cargo.lock. Binary crates
# should commit the lockfile for reproducible deployments.
lock:
        cargo generate-lockfile
        @echo "Cargo.lock generated — commit it for reproducible deployments."
