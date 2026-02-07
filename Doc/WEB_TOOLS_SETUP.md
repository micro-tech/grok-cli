# Web Tools Setup Guide

## Overview

Grok CLI includes web search and web fetch tools that allow the AI to search the web and fetch content from URLs during conversations. These tools require configuration before use.

## Available Web Tools

### 1. web_search
Search the web using Google Custom Search API.

**Capabilities:**
- Search Google for relevant information
- Get up-to-date information not in training data
- Find documentation, tutorials, and solutions
- Research topics during conversations

### 2. web_fetch
Fetch and read content from any URL.

**Capabilities:**
- Download web page content
- Read API responses
- Fetch documentation from URLs
- Access online resources

## Setup Instructions

### Prerequisites

To use web search, you need:
1. A Google Cloud Platform account
2. A Google Custom Search Engine
3. API credentials (free tier available)

### Step 1: Get Google API Key

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project (or select existing)
3. Enable the **Custom Search API**:
   - Navigate to "APIs & Services" > "Library"
   - Search for "Custom Search API"
   - Click "Enable"
4. Create API credentials:
   - Go to "APIs & Services" > "Credentials"
   - Click "Create Credentials" > "API Key"
   - Copy the API key (keep it secure!)
5. (Optional) Restrict the API key:
   - Click on the key to edit
   - Under "API restrictions", select "Custom Search API"
   - Save

### Step 2: Create Custom Search Engine

1. Go to [Google Custom Search Engine](https://cse.google.com/cse/)
2. Click "Add" to create a new search engine
3. Configuration:
   - **Sites to search**: Enter `www.google.com` (or leave blank to search entire web)
   - **Name**: Give it a descriptive name (e.g., "Grok CLI Search")
   - **Search the entire web**: Toggle ON (recommended)
4. Click "Create"
5. Get your Search Engine ID:
   - Click on your new search engine
   - Go to "Setup" > "Basics"
   - Copy the "Search engine ID" (starts with a number)

### Step 3: Configure Environment Variables

#### Windows (PowerShell)

**Temporary (current session only):**
```powershell
$env:GOOGLE_API_KEY="your_api_key_here"
$env:GOOGLE_CX="your_search_engine_id_here"
```

**Permanent (using System Properties):**
1. Open System Properties (Win + Pause/Break)
2. Click "Advanced system settings"
3. Click "Environment Variables"
4. Under "User variables", click "New"
5. Add:
   - Variable name: `GOOGLE_API_KEY`
   - Variable value: Your API key
6. Repeat for `GOOGLE_CX`
7. Click OK and restart your terminal

**Permanent (using PowerShell profile):**
```powershell
# Edit your PowerShell profile
notepad $PROFILE

# Add these lines:
$env:GOOGLE_API_KEY="your_api_key_here"
$env:GOOGLE_CX="your_search_engine_id_here"

# Save and reload
. $PROFILE
```

#### macOS/Linux (Bash/Zsh)

**Temporary (current session only):**
```bash
export GOOGLE_API_KEY="your_api_key_here"
export GOOGLE_CX="your_search_engine_id_here"
```

**Permanent:**
```bash
# Add to ~/.bashrc, ~/.zshrc, or ~/.profile
echo 'export GOOGLE_API_KEY="your_api_key_here"' >> ~/.bashrc
echo 'export GOOGLE_CX="your_search_engine_id_here"' >> ~/.bashrc

# Reload configuration
source ~/.bashrc
```

#### Using .env File (Recommended for Projects)

Create a `.env` file in your project root:

```env
GOOGLE_API_KEY=your_api_key_here
GOOGLE_CX=your_search_engine_id_here
```

Grok CLI will automatically load this file.

**Important:** Add `.env` to your `.gitignore` to keep credentials secure!

### Step 4: Verify Configuration

Test that everything works:

```bash
grok interactive

> Ask Grok to search the web
> "Can you search for the latest Rust release notes?"
```

If configured correctly, Grok will use the web_search tool to find current information.

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

### "GOOGLE_API_KEY environment variable not set"

**Problem:** API credentials not configured.

**Solution:**
1. Check if environment variables are set:
   ```bash
   # Windows PowerShell
   echo $env:GOOGLE_API_KEY
   echo $env:GOOGLE_CX
   
   # macOS/Linux
   echo $GOOGLE_API_KEY
   echo $GOOGLE_CX
   ```
2. If empty, follow Step 3 above
3. Restart your terminal after setting variables

### "Search request failed: 403"

**Problem:** API key is invalid or Custom Search API not enabled.

**Solution:**
1. Verify API key is correct
2. Ensure Custom Search API is enabled in Google Cloud Console
3. Check API key restrictions aren't blocking requests
4. Verify billing is enabled (required even for free tier)

### "Search request failed: 429"

**Problem:** API rate limit exceeded.

**Solution:**
- Google Custom Search free tier: 100 queries/day
- Wait 24 hours for quota reset
- Or upgrade to paid tier in Google Cloud Console

### "No results found"

**Problem:** Search engine not configured to search entire web.

**Solution:**
1. Go to [Google CSE](https://cse.google.com/cse/)
2. Edit your search engine
3. Enable "Search the entire web"
4. Save changes

### "Failed to fetch URL: Network error"

**Problem:** Network connectivity or firewall issues.

**Solution:**
1. Check internet connection
2. Test URL in browser first
3. Check if behind corporate firewall/proxy
4. Verify URL is correct and accessible
5. Some sites block automated requests

### Web Tools Not Appearing

**Problem:** Tools filtered out when credentials not configured.

**Solution:**
- Grok CLI automatically hides web_search if not configured
- Set up Google API credentials (see above)
- Tools will appear once properly configured

## API Costs

### Google Custom Search API Pricing

**Free Tier:**
- 100 queries per day
- $0 cost
- No credit card required initially

**Paid Tier:**
- $5 per 1,000 queries
- $0.005 per query after free tier
- First 100 queries free each day

**Billing Setup:**
- Free tier requires enabling billing (won't be charged)
- Paid usage only if you exceed 100 queries/day

### Cost Management Tips

1. **Monitor Usage:**
   - Check [Google Cloud Console](https://console.cloud.google.com/) > "APIs & Services" > "Quotas"
   
2. **Set Quotas:**
   - Limit daily queries to avoid unexpected charges
   - Set budget alerts in Google Cloud Console

3. **Optimize Queries:**
   - Be specific with search terms
   - Use web_fetch for known URLs instead of searching
   - Cache results when possible

## Security Best Practices

### Protecting API Keys

1. **Never commit credentials to git:**
   ```gitignore
   .env
   *.env
   ```

2. **Use environment variables:**
   - Don't hardcode in scripts
   - Use .env files for local development
   - Use proper secrets management in production

3. **Restrict API keys:**
   - Limit to specific APIs (Custom Search API only)
   - Set application restrictions if possible
   - Regenerate keys if exposed

4. **Monitor usage:**
   - Check Cloud Console regularly
   - Enable billing alerts
   - Review access logs

## Alternative: Disable Web Tools

If you don't need web functionality:

**Option 1:** Simply don't configure the API keys
- Web tools automatically filtered out
- No error messages
- Other tools work normally

**Option 2:** Use without web features
- Focus on local file operations
- Code generation and analysis
- Project-specific assistance

## FAQ

### Do I need a credit card?

No credit card required for the free tier. Google may ask you to enable billing, but won't charge unless you exceed free quota.

### Can I use a different search engine?

Currently only Google Custom Search is supported. The implementation could be extended to support other search APIs.

### Does web_fetch work without Google API?

Yes! `web_fetch` works independently and doesn't require any API keys. Only `web_search` needs Google credentials.

### Are there usage limits for web_fetch?

No API limits, but:
- Network timeouts (30 seconds)
- Some sites block automated requests
- Respect rate limits of target sites

### Can I search private/internal sites?

Yes, configure your Custom Search Engine to include specific domains:
1. Edit your CSE settings
2. Add specific sites to search
3. Optionally disable "Search the entire web"

### Is my search history private?

Yes:
- Searches go through your own Google API key
- Not shared with other users
- Subject to Google's API terms of service

## Support

If you encounter issues:

1. Check error messages - they include helpful setup instructions
2. Verify environment variables are set
3. Test API key at [Google Cloud Console](https://console.cloud.google.com/)
4. Review [Google Custom Search documentation](https://developers.google.com/custom-search)
5. Report bugs on [GitHub Issues](https://github.com/microtech/grok-cli/issues)

## Summary

**Quick Setup Checklist:**

- [ ] Create Google Cloud Platform project
- [ ] Enable Custom Search API
- [ ] Generate API key
- [ ] Create Custom Search Engine
- [ ] Get Search Engine ID
- [ ] Set GOOGLE_API_KEY environment variable
- [ ] Set GOOGLE_CX environment variable
- [ ] Restart terminal
- [ ] Test with `grok interactive`

Once configured, Grok can search the web and fetch URLs to provide current information beyond its training data!