#!/bin/bash
set -e

# Spinner function - shows activity while claude runs
spinner() {
  local pid=$1
  local delay=1
  local elapsed=0

  while ps -p $pid > /dev/null 2>&1; do
    printf "\r   ⏳ Claude is thinking... [%dm %ds elapsed]" $((elapsed/60)) $((elapsed%60))
    sleep $delay
    elapsed=$((elapsed + 1))
  done
  printf "\r   ✅ Claude finished thinking                    \n"
}

if [ -z "$1" ]; then
  echo "Usage: $0 <iterations>"
  exit 1
fi

echo "=========================================="
echo "🤖 RALPH - AFK Mode"
echo "=========================================="
echo "📋 PRD: prd.md"
echo "📝 Progress: progress.txt"
echo "🔄 Max iterations: $1"
echo "⏰ Started: $(date)"
echo "=========================================="

for ((i=1; i<=$1; i++)); do
  echo ""
  echo "------------------------------------------"
  echo "📍 Iteration $i of $1"
  echo "⏰ $(date)"
  echo "------------------------------------------"
  echo ""

  # Create temp file for output
  tmpfile=$(mktemp)

  # Run claude in background
  claude --dangerously-skip-permissions -p "@prd.md @progress.txt \
  1. Find the highest-priority task and implement it. \
  2. Run your tests and type checks. \
  3. Update the PRD with what was done. \
  4. Append your progress to progress.txt. \
  5. Commit your changes. \
  ONLY WORK ON A SINGLE TASK. \
  If the PRD is complete, output <promise>COMPLETE</promise>." > "$tmpfile" 2>&1 &

  claude_pid=$!

  # Show spinner while waiting
  spinner $claude_pid

  # Wait for claude to finish and get exit code
  wait $claude_pid || true
  exit_code=$?

  # Read the result
  result=$(cat "$tmpfile")
  rm "$tmpfile"

  # Show what Claude did
  echo ""
  echo "📄 Claude's output:"
  echo "---"
  echo "$result"
  echo "---"

  if [[ "$result" == *"<promise>COMPLETE</promise>"* ]]; then
    echo ""
    echo "=========================================="
    echo "🎉 PRD COMPLETE!"
    echo "✅ Finished after $i iterations"
    echo "⏰ $(date)"
    echo "=========================================="
    exit 0
  fi

  if [ $exit_code -ne 0 ]; then
    echo ""
    echo "⚠️  Claude exited with error code $exit_code"
  fi

  echo ""
  echo "✅ Iteration $i complete"
done

echo ""
echo "=========================================="
echo "⚠️  Reached max iterations ($1)"
echo "📝 Check progress.txt for status"
echo "⏰ Finished: $(date)"
echo "=========================================="
