#!/usr/bin/env bash
# PreToolUse hook on Bash. Denies a push to a protected branch, any force push,
# merging an MR or PR, and the destructive git commands. Everything else passes.
# Protected branches come from the repository profile, or the default list.

set -uo pipefail

input="$(cat)"
cmd="$(jq -r '.tool_input.command // ""' <<<"$input")"
cwd="$(jq -r '.cwd // empty' <<<"$input")"
cwd="${cwd:-$PWD}"

deny() {
  echo "ticket-flow guard: $1. A human does this." >&2
  exit 2
}

# shellcheck source=lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

protected=()
load_protected() {
  local profile line item
  if profile="$(profile_path "$cwd")"; then
    line="$(profile_get "$profile" protected_branches)"
    line="${line#*[}"
    line="${line%]*}"
    IFS=',' read -ra items <<<"$line"
    for item in "${items[@]}"; do
      item="$(tr -d " \"'" <<<"$item")"
      [[ -n "$item" ]] && protected+=("$item")
    done
  fi
  if [[ ${#protected[@]} -eq 0 ]]; then
    protected=(main master develop staging prod production 'release/*')
  fi
}

is_protected() {
  local branch="${1#refs/heads/}" pattern
  for pattern in "${protected[@]}"; do
    # shellcheck disable=SC2053 # the pattern is a glob on purpose, like release/*
    [[ "$branch" == $pattern ]] && return 0
  done
  return 1
}

current_branch() {
  git -C "$1" rev-parse --abbrev-ref HEAD 2>/dev/null || echo HEAD
}

# Checks one `git ... push ...` segment, already split into words.
check_push() {
  local dir="$cwd" words=("$@") i=0 w
  # Global options before the subcommand: -C <dir>, -c <k=v>, and flags.
  while ((i < ${#words[@]})); do
    w="${words[i]}"
    case "$w" in
      git) ;;
      -C) i=$((i + 1)); dir="${words[i]}"; [[ "$dir" = /* ]] || dir="$cwd/$dir" ;;
      -c) i=$((i + 1)) ;;
      -*) ;;
      push) break ;;
      *) return 0 ;;
    esac
    i=$((i + 1))
  done
  [[ "${words[i]:-}" == push ]] || return 0

  local positional=() skip_next=0
  for w in "${words[@]:i+1}"; do
    if ((skip_next)); then skip_next=0; continue; fi
    case "$w" in
      --force | --force-with-lease* | --force-if-includes) deny "force push is not allowed" ;;
      --all | --mirror) deny "push --all and --mirror are not allowed" ;;
      -o | --push-option | --repo | --receive-pack | --exec) skip_next=1 ;;
      --*) ;;
      -*) [[ "$w" == *f* ]] && deny "force push is not allowed" ;;
      *) positional+=("$w") ;;
    esac
  done

  local refspecs=("${positional[@]:1}") ref dst
  if [[ ${#refspecs[@]} -eq 0 ]]; then
    refspecs=("$(current_branch "$dir")")
  fi
  for ref in "${refspecs[@]}"; do
    [[ "$ref" == +* ]] && deny "force push is not allowed"
    dst="${ref##*:}"
    [[ -z "$dst" || "$dst" == HEAD ]] && dst="$(current_branch "$dir")"
    is_protected "$dst" && deny "pushing to the protected branch $dst is not allowed"
  done
  return 0
}

[[ "$cmd" =~ (glab|gh)[[:space:]]+(mr|pr)[[:space:]]+merge ]] && deny "merging is not allowed"
[[ "$cmd" == *--auto-merge* ]] && deny "auto-merge is not allowed"
[[ "$cmd" =~ reset[[:space:]]+--hard ]] && deny "git reset --hard is not allowed"
[[ "$cmd" =~ clean[[:space:]]+-[a-zA-Z]*f ]] && deny "git clean -f is not allowed"

if [[ "$cmd" == *push* ]]; then
  load_protected
  # shortcut: splits on shell operators without parsing quotes, enough for the commands agents write.
  while IFS= read -r segment; do
    read -ra words <<<"$segment"
    [[ " ${words[*]} " == *" git "* && " ${words[*]} " == *" push"* ]] || continue
    # Start at the git word, so `cd x && git push` and `FOO=1 git push` both work.
    for ((j = 0; j < ${#words[@]}; j++)); do
      [[ "${words[j]}" == git ]] && { check_push "${words[@]:j}"; break; }
    done
  done < <(sed -E 's/(&&|\|\||;|\|)/\n/g' <<<"$cmd")
fi

exit 0
