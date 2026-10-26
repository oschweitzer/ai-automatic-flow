#!/usr/bin/env bash
# The only place that talks to GitLab or GitHub. Skills call it, never glab or
# gh directly, so supporting another forge means changing this file only.
# Run it from inside the repository. See usage below.

set -euo pipefail

# shellcheck source=lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

usage() {
  cat <<'EOF'
Usage: forge.sh <command> [args]

  kind                                  gitlab or github
  whoami                                login of the authenticated user
  find-mr <branch>                      "<number>\t<url>" of the open MR, or nothing
  open-mr <branch> <base> <title> <body-file>
                                        open a draft MR, print "<number>\t<url>"
  ci-status <mr>                        success | failed | running | blocked | none
  ci-failed-log <mr> [lines]            tail of each failed job's log
  post-thread <mr> <path> <line> <body-file>
                                        open a thread on a changed line, print its id
  list-threads <mr>                     JSON array of threads, see comments.jq
  reply <mr> <thread> <body-file>       reply in a thread
  resolve <mr> <thread>                 resolve a thread
  upsert-note <mr> <marker> <body-file> update the note containing marker, or post it
  upload <mr> <file>                    print a Markdown link to the uploaded file
  mark-ready <mr>                       take the MR out of draft
EOF
}

forge_kind() {
  local profile kind=""
  if profile="$(profile_path "$PWD")"; then
    kind="$(profile_get "$profile" forge)"
  fi
  if [[ -z "$kind" ]]; then
    case "$(git remote get-url origin 2>/dev/null)" in
      *github.com*) kind=github ;;
      *) kind=gitlab ;;
    esac
  fi
  echo "$kind"
}

# ---------------------------------------------------------------- GitLab

gl_mr() { echo "projects/:id/merge_requests/$1"; }

gl_whoami() { glab api user | jq -r .username; }

gl_find_mr() {
  glab api --method GET projects/:id/merge_requests -f state=opened -f source_branch="$1" |
    jq -r '.[0] // empty | "\(.iid)\t\(.web_url)"'
}

gl_open_mr() {
  glab mr create --draft --source-branch "$1" --target-branch "$2" \
    --title "$3" --description-file "$4" --yes >/dev/null
  gl_find_mr "$1"
}

gl_ci_status() {
  glab api "$(gl_mr "$1")" | jq -r '
    (.head_pipeline.status // "") as $s
    | if $s == "" then "none"
      elif $s == "success" then "success"
      elif $s == "failed" or $s == "canceled" then "failed"
      elif $s == "manual" or $s == "skipped" then "blocked"
      else "running" end'
}

gl_ci_failed_log() {
  local mr="$1" lines="${2:-80}" pipeline id name
  pipeline="$(glab api "$(gl_mr "$mr")" | jq -r '.head_pipeline.id // empty')"
  [[ -n "$pipeline" ]] || { echo "no pipeline on MR $mr"; return 0; }
  glab api --method GET "projects/:id/pipelines/$pipeline/jobs" -f 'scope[]=failed' |
    jq -r '.[] | "\(.id)\t\(.name)"' |
    while IFS=$'\t' read -r id name; do
      echo "===== job $name (last $lines lines)"
      glab api "projects/:id/jobs/$id/trace" | sed -E 's/\x1b\[[0-9;]*[a-zA-Z]//g' | tail -n "$lines"
    done
}

gl_post_thread() {
  local mr="$1" path="$2" line="$3" body="$4" position
  position="$(glab api "$(gl_mr "$mr")/versions" | jq -c --arg p "$path" --argjson l "$line" '.[0] | {
      position_type: "text", base_sha: .base_commit_sha, head_sha: .head_commit_sha,
      start_sha: .start_commit_sha, old_path: $p, new_path: $p, new_line: $l }')"
  # A line GitLab cannot place (unchanged context, renamed file) is refused,
  # so fall back to a thread on the MR that names the location.
  glab api --method POST "$(gl_mr "$mr")/discussions" -F "body=@$body" -F "position=$position" 2>/dev/null |
    jq -r .id && return 0
  local fallback
  fallback="$(mktemp)"
  { echo "\`$path:$line\`"; echo; cat "$body"; } >"$fallback"
  glab api --method POST "$(gl_mr "$mr")/discussions" -F "body=@$fallback" | jq -r .id
  rm -f "$fallback"
}

gl_list_threads() {
  glab api --paginate "$(gl_mr "$1")/discussions" | jq '[
    .[] | select(.notes[0].system | not) | {
      thread: .id,
      resolvable: (.notes[0].resolvable // false),
      resolved: (.notes[0].resolved // false),
      path: (.notes[0].position.new_path // null),
      line: (.notes[0].position.new_line // null),
      notes: [.notes[] | select(.system | not) | {id: (.id | tostring), author: .author.username, body, at: .created_at}]
    }]'
}

gl_reply() {
  glab api --method POST "$(gl_mr "$1")/discussions/$2/notes" -F "body=@$3" | jq -r .id
}

gl_resolve() {
  glab api --method PUT "$(gl_mr "$1")/discussions/$2" -F resolved=true >/dev/null
}

gl_upsert_note() {
  local mr="$1" marker="$2" body="$3" id
  id="$(glab api --paginate "$(gl_mr "$mr")/notes" |
    jq -r --arg m "$marker" '[.[] | select(.body | contains($m))][0].id // empty')"
  if [[ -n "$id" ]]; then
    glab api --method PUT "$(gl_mr "$mr")/notes/$id" -F "body=@$body" | jq -r .id
  else
    glab api --method POST "$(gl_mr "$mr")/notes" -F "body=@$body" | jq -r .id
  fi
}

gl_upload() {
  glab api --method POST projects/:id/uploads --form "file=@$2" | jq -r .markdown
}

gl_mark_ready() { glab mr update "$1" --ready >/dev/null; }

# ---------------------------------------------------------------- GitHub
# Not tried yet: GitLab comes first. See verification.md.

gh_repo() { gh repo view --json nameWithOwner -q .nameWithOwner; }

gh_whoami() { gh api user -q .login; }

gh_find_mr() {
  gh pr list --head "$1" --state open --json number,url | jq -r '.[0] // empty | "\(.number)\t\(.url)"'
}

gh_open_mr() {
  gh pr create --draft --head "$1" --base "$2" --title "$3" --body-file "$4" >/dev/null
  gh_find_mr "$1"
}

gh_ci_status() {
  # gh pr checks exits non-zero while checks fail or run, the JSON is still printed.
  (gh pr checks "$1" --json bucket 2>/dev/null || true) | jq -r '
    [.[].bucket] as $b
    | if ($b | length) == 0 then "none"
      elif any($b[]; . == "fail" or . == "cancel") then "failed"
      elif any($b[]; . == "pending") then "running"
      else "success" end'
}

gh_ci_failed_log() {
  local mr="$1" lines="${2:-80}" run
  (gh pr checks "$mr" --json bucket,link 2>/dev/null || true) |
    jq -r '.[] | select(.bucket == "fail") | .link | capture("/runs/(?<id>[0-9]+)").id' | sort -u |
    while read -r run; do
      echo "===== run $run (last $lines lines)"
      gh run view "$run" --log-failed | tail -n "$lines"
    done
}

GH_THREADS_QUERY='query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) { pullRequest(number: $number) {
    reviewThreads(first: 100) { nodes { id isResolved path line
      comments(first: 50) { nodes { databaseId author { login } body createdAt } } } } } } }'

gh_list_threads() {
  local repo
  repo="$(gh_repo)"
  gh api graphql -f query="$GH_THREADS_QUERY" -f owner="${repo%/*}" -f name="${repo#*/}" -F number="$1" | jq '[
    .data.repository.pullRequest.reviewThreads.nodes[] | {
      thread: .id, resolvable: true, resolved: .isResolved, path, line,
      notes: [.comments.nodes[] | {id: (.databaseId | tostring), author: .author.login, body, at: .createdAt}]
    }]'
}

gh_post_thread() {
  local mr="$1" path="$2" line="$3" body="$4" repo sha id
  repo="$(gh_repo)"
  sha="$(gh pr view "$mr" --json headRefOid -q .headRefOid)"
  if id="$(gh api "repos/$repo/pulls/$mr/comments" -f body="$(cat "$body")" -f commit_id="$sha" \
    -f path="$path" -F line="$line" -f side=RIGHT -q .id 2>/dev/null)"; then
    gh_list_threads "$mr" | jq -r --arg id "$id" '.[] | select(.notes[0].id == $id) | .thread'
  else
    gh pr comment "$mr" --body "$(printf '`%s:%s`\n\n%s' "$path" "$line" "$(cat "$body")")" >/dev/null
    echo "-"
  fi
}

gh_reply() {
  local mr="$1" thread="$2" body="$3" repo first
  repo="$(gh_repo)"
  first="$(gh_list_threads "$mr" | jq -r --arg t "$thread" '.[] | select(.thread == $t) | .notes[0].id')"
  gh api "repos/$repo/pulls/$mr/comments/$first/replies" -f body="$(cat "$body")" -q .id
}

gh_resolve() {
  gh api graphql -f query='mutation($id: ID!) { resolveReviewThread(input: {threadId: $id}) { thread { isResolved } } }' \
    -f id="$2" >/dev/null
}

gh_upsert_note() {
  local mr="$1" marker="$2" body="$3" repo id
  repo="$(gh_repo)"
  id="$(gh api --paginate "repos/$repo/issues/$mr/comments" |
    jq -rs --arg m "$marker" '[.[][] | select(.body | contains($m))][0].id // empty')"
  if [[ -n "$id" ]]; then
    gh api --method PATCH "repos/$repo/issues/comments/$id" -f body="$(cat "$body")" -q .id
  else
    gh api "repos/$repo/issues/$mr/comments" -f body="$(cat "$body")" -q .id
  fi
}

# GitHub has no documented upload for comments, so files go to an orphan
# branch that is never merged. The git guard allows pushing to it.
gh_upload() {
  local mr="$1" file="$2" repo root wt name
  repo="$(gh_repo)"
  root="$(git rev-parse --show-toplevel)"
  wt="$root/.ticket-flow/assets-worktree"
  name="$(basename "$file")"
  if [[ ! -d "$wt" ]]; then
    git -C "$root" worktree add --quiet --detach "$wt" HEAD
    if git -C "$root" fetch --quiet origin ticket-flow-assets 2>/dev/null; then
      git -C "$wt" checkout --quiet -B ticket-flow-assets FETCH_HEAD
    else
      git -C "$wt" checkout --quiet --orphan ticket-flow-assets
      git -C "$wt" rm -rfq .
    fi
  fi
  mkdir -p "$wt/pr-$mr"
  cp "$file" "$wt/pr-$mr/$name"
  git -C "$wt" add "pr-$mr/$name"
  git -C "$wt" commit --quiet -m "ticket-flow assets for #$mr"
  git -C "$wt" push --quiet origin ticket-flow-assets
  echo "[$name](https://github.com/$repo/blob/ticket-flow-assets/pr-$mr/$name?raw=true)"
}

gh_mark_ready() { gh pr ready "$1" >/dev/null; }

# ---------------------------------------------------------------- dispatch

main() {
  local command="${1:-}" kind fn
  [[ -n "$command" && "$command" != -h && "$command" != --help ]] || { usage; exit 0; }
  shift
  kind="$(forge_kind)"
  [[ "$command" == kind ]] && { echo "$kind"; exit 0; }
  case "$kind" in
    gitlab) fn="gl_${command//-/_}" ;;
    github) fn="gh_${command//-/_}" ;;
    *) echo "forge.sh: unknown forge \"$kind\" in the profile" >&2; exit 1 ;;
  esac
  declare -F "$fn" >/dev/null || { echo "forge.sh: unknown command $command" >&2; usage >&2; exit 1; }
  "$fn" "$@"
}

main "$@"
