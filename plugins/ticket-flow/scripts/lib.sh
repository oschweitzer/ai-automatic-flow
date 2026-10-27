# Helpers shared by the ticket-flow scripts. Source it, do not run it.

# Prints the path of the repository profile for the git repository at $1.
# In a worktree the profile may not be committed yet, so fall back to the
# main checkout.
profile_path() {
  local dir="${1:-$PWD}" root common
  root="$(git -C "$dir" rev-parse --show-toplevel 2>/dev/null)" || return 1
  if [[ -f "$root/.claude/ticket-flow.md" ]]; then
    echo "$root/.claude/ticket-flow.md"
    return 0
  fi
  common="$(git -C "$dir" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" || return 1
  [[ -f "$(dirname "$common")/.claude/ticket-flow.md" ]] || return 1
  echo "$(dirname "$common")/.claude/ticket-flow.md"
}

# Prints the ticket tracker of the profile at $1: jira when the key is missing.
profile_tracker() {
  local tracker
  tracker="$(profile_get "$1" tracker)"
  echo "${tracker:-jira}"
}

# Prints the value of a frontmatter key from the profile at $1. A nested key
# is written parent.child, like commands.test. Quotes and trailing comments
# are stripped.
# shortcut: reads flat keys and one level of nesting only, enough for the profile format.
profile_get() {
  local file="$1" key="$2"
  awk -v key="$key" '
    NR == 1 && /^---$/ { inside = 1; next }
    inside && /^---$/ { exit }
    !inside { next }
    { name = "" }
    /^[^ #][^:]*:/ { parent = $0; sub(/:.*/, "", parent); name = parent }
    /^  [^ #-][^:]*:/ { name = $0; sub(/^ +/, "", name); sub(/:.*/, "", name); name = parent "." name }
    name == key {
      value = $0
      sub(/^[^:]*:[ \t]*/, "", value)
      sub(/[ \t]+#.*$/, "", value)
      gsub(/^["\047]|["\047]$/, "", value)
      print value
      exit
    }' "$file"
}
