# Substreams Assignment: Solana Transfer Extraction

## Time Limit: 2 Hours

---

## IMPORTANT: Read This First

**This is an extremely difficult assignment.**

You may have never heard of blockchain, Rust, Substreams, or Solana before. Most candidates haven't. That's intentional.

**We WANT you to use AI tools.** Use ChatGPT, Claude, Copilot, Gemini, or any AI assistant you prefer. Use Google, Stack Overflow, GitHub, official docs—whatever helps you succeed.

**Why are we doing this?**

We're evaluating how you perform when faced with a hard, ambiguous task in unfamiliar territory. We want to see:
- How you break down a complex problem
- How effectively you leverage AI and other resources
- How you persist through challenges
- Your problem-solving approach

**Making good progress on this assignment often means we instantly hire you.** This is not an exaggeration—candidates who demonstrate strong problem-solving with unfamiliar technologies stand out significantly.

**All we need from you: Don't give up. Try your best.**

Your first goal is to **make sense of the assignment**, then **execute it with AI assistance**.

---

## Overview

Your goal is to create a **Substreams SPKG file** that extracts transfer actions from the Solana blockchain.

You must extract transfers for **Solana blocks 385870151 to 385870156**.

---

## Required Output Format

Each transfer message must be in this exact JSON format:

```json
{"from": "Eq1c3uNugn3FRpJ99K2q1ZyQrora3jntUqvkMnGroKkB", "to": "4XpSNfoNfireSgdg8hY6Ng3dWrHjewZhmCancypjbcCN", "amount": 325253}
```

Where:
- `from` - The sender's Solana wallet address (base58 encoded)
- `to` - The recipient's Solana wallet address (base58 encoded)
- `amount` - The transfer amount (as an integer)

**Important:** A single block can contain **multiple transfers**. Your output should include one JSON object per transfer found in each block. For example, if block 385870151 has 5 transfers, you should output 5 separate JSON messages for that block.

---

## Accessing the Terminal in Code-Server

You are working in a Docker environment with Code-Server as your IDE. Here's how to access the terminal:

### Method 1: Keyboard Shortcut
Press `` Ctrl + ` `` (Control + Backtick) to toggle the integrated terminal

### Method 2: Menu Navigation
1. Click on the **hamburger menu** (three horizontal lines) in the top-left corner
2. Navigate to **Terminal** → **New Terminal**

### Method 3: Command Palette
1. Press `Ctrl + Shift + P` to open the Command Palette
2. Type `Terminal: Create New Terminal`
3. Press Enter

Once the terminal is open, you'll see a bash prompt at the bottom of your screen where you can run commands.

---

## What You Need To Do

### Step 1: Install Rust

You must use **Rust** to build the Substreams SPKG package—this is mandatory.

We understand you may have never heard of Rust before. That's exactly the point. We want to see how you thrive in an unknown language using AI tools. Many successful candidates have completed this assignment without any prior Rust experience.

Figure out how to install Rust on your system.

### Step 2: Install the Substreams CLI

Go to the Substreams documentation and figure out how to install the CLI.

**We evaluate whether you can navigate documentation and figure out installation on your own.**

### Step 3: Get a Free API Key

You need an API key to stream data from a Substreams provider. Figure out how to obtain one.

### Step 4: Develop Your Substreams Module

You need to create a Rust-based Substreams module that:
1. Processes Solana blocks
2. Extracts transfer instructions
3. Outputs each transfer in the required format

This will involve:
- Setting up a Rust project
- Creating a `substreams.yaml` manifest
- Writing protobuf definitions
- Implementing your map module in Rust

### Step 5: Build the SPKG File

Package your module into an SPKG file.

### Step 6: Run and Verify

Run your Substreams against blocks **385870151 to 385870156** and verify the output matches the required format.

---

## Success Criteria

To successfully complete this assignment, you must:

1. **Build a valid SPKG file** - The package must compile without errors
2. **Run the Substreams** - Successfully execute against blocks 385870151-385870156
3. **Output correct format** - Each transfer must be formatted as:
   ```json
   {"from": "Eq1c3uNugn3FRpJ99K2q1ZyQrora3jntUqvkMnGroKkB", "to": "4XpSNfoNfireSgdg8hY6Ng3dWrHjewZhmCancypjbcCN", "amount": 325253}
   ```

---

## Tips for Success

1. **Start with understanding** - Spend time reading documentation before coding
2. **Use AI effectively** - Ask AI tools to explain concepts, debug errors, and suggest code
3. **Work incrementally** - Get a basic version working first, then refine
4. **Read error messages** - They often tell you exactly what's wrong

---

## Final Notes

Remember: We're not expecting perfection. We want to see your problem-solving process, how you handle ambiguity, and how effectively you use available tools and resources.

**Don't give up. Try your best. Good luck!**
