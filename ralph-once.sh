#!/bin/bash

echo "=========================================="
echo "🤖 RALPH - Single Task Mode"
echo "=========================================="
echo "📋 PRD: prd.md"
echo "📝 Progress: progress.txt"
echo "⏰ Started: $(date)"
echo "=========================================="
echo ""
echo "🔍 Finding next task and implementing..."
echo "💡 Claude is running interactively - you should see output below"
echo ""

claude --permission-mode acceptEdits "@prd.md @progress.txt \
1. Read the PRD and progress file. \
2. Find the next incomplete task and implement it. \
3. Commit your changes. \
4. Update progress.txt with what you did. \
ONLY DO ONE TASK AT A TIME."

echo ""
echo "=========================================="
echo "✅ Ralph iteration complete"
echo "⏰ Finished: $(date)"
echo "=========================================="
