FROM rust:1.98.1 AS builder

# Set working directory
WORKDIR /app

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Create src directory & add main.rs file
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release

# Now copy the source code
COPY src ./src

# Build application
RUN touch src/main.rs && cargo build --release

# Start a new stage
FROM debian:12.15-slim

# Set working directory
WORKDIR /app

# Copy application from previous stage
COPY --from=builder /app/target/release/livekit-token-server ./

# Run the application
CMD ["./livekit-token-server"]
