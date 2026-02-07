#!/bin/bash
echo "Testing .env loading..."
echo "GOOGLE_API_KEY in .grok/.env:"
grep GOOGLE_API_KEY .grok/.env
echo ""
echo "Checking if grok loads it:"
cargo run --quiet -- --version 2>&1 | head -1
