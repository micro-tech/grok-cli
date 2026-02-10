#!/bin/bash
echo "Testing .env loading..."
echo "GROK_API_KEY in .grok/.env:"
grep GROK_API_KEY .grok/.env
echo ""
echo "Checking if grok loads it:"
cargo run --quiet -- --version 2>&1 | head -1
