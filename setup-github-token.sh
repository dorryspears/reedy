# Method 1: Temporary export of GitHub token
export GITHUB_TOKEN="<your-token-here>"  # Replace with your actual token

# Method 2: Add to .zshrc for persistence
echo "export GITHUB_TOKEN=<your-token-here>" >> ~/.zshrc  # Replace with your actual token
source ~/.zshrc  # Reload configuration
