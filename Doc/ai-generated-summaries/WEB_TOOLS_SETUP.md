# Web Tools Setup Guide

## Overview

Grok CLI includes web search and web fetch tools that allow the AI to search the web and fetch content from URLs during conversations.

## Available Web Tools

### 1. web_search
Search the web using DuckDuckGo.

**Capabilities:**
- Search for relevant information
- Get up-to-date information not in training data
- Find documentation, tutorials, and solutions
- Research topics during conversations
- **No configuration required**

### 2. web_fetch
Fetch and read content from any URL.

**Capabilities:**
- Download web page content
- Read API responses
- Fetch documentation from URLs
- Access online resources

## Setup Instructions

**Good news! No setup is required.** 

Both `web_search` (via DuckDuckGo) and `web_fetch` work out of the box without any API keys or configuration.

### Verify Configuration

Test that everything works:

```bash
grok interactive

> Ask Grok to search the web
> "Can you search for the latest Rust release notes?"
```

Grok will use the web_search tool to find current information.

## Usage Examples

### Web Search

```bash
grok interactive

> What are the latest features in Rust 1.75?
# Grok will search the web for current information

> Search for best practices for async Rust in 2024
# Gets up-to-date community knowledge
```

### Web Fetch

```bash
> Can you fetch and summarize https://doc.rust-lang.org/book/
# Grok fetches and reads the content

> Get the content from this API: https://api.example.com/data
# Fetches and parses API responses
```

## Troubleshooting

### "Failed to fetch URL: Network error"

**Problem:** Network connectivity or firewall issues.

**Solution:**
1. Check internet connection
2. Test URL in browser first
3. Check if behind corporate firewall/proxy
4. Verify URL is correct and accessible
5. Some sites block automated requests

### "DuckDuckGo search failed"

**Problem:** DuckDuckGo might be temporarily unavailable or blocking requests.

**Solution:**
- Wait a few minutes and try again.
- Check internet connection.

## FAQ

### Do I need an API key?

No. DuckDuckGo search is free and does not require an API key.

### Does web_fetch work without configuration?

Yes! `web_fetch` works independently and doesn't require any API keys.

### Are there usage limits?

No hard API limits, but:
- Network timeouts (30 seconds)
- Some sites block automated requests
- Respect rate limits of target sites

### Can I search private/internal sites?

No, the built-in `web_search` uses DuckDuckGo public search. For internal sites, you would need to use `web_fetch` with specific URLs.