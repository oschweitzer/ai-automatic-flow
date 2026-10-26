# Turns the GraphQL answer of watch-pr-reviews.sh into one TSV row per review
# comment worth acting on:
#   id, pr, branch, kind, thread, location, author, url
#
# Kept: comments in unresolved threads, and review bodies that are not empty.
# Dropped: anything written by the viewer (your own replies) or by a login in
# $ignore, and when $only is not empty, anything by a login not in it. Every
# field is non-empty, because bash `read` merges empty TSV fields.

def author: .author.login // "ghost";
def skipped($skip): . as $author
  | ($skip | any(. == $author)) or (($only | length) > 0 and ($only | any(. == $author) | not));

.data.viewer.login as $me
| ($ignore + [$me]) as $skip
| .data.search.nodes[]
| select(.number != null)
| . as $pr
| (
    ( $pr.reviewThreads.nodes[]
      | select(.isResolved | not)
      | . as $thread
      | .comments.nodes[]
      | author as $author
      | select($author | skipped($skip) | not)
      | [ .id, $pr.number, $pr.headRefName, "thread", $thread.id,
          "\($thread.path):\($thread.line // "outdated")", $author, .url ] ),
    ( $pr.reviews.nodes[]
      | author as $author
      | select(($author | skipped($skip) | not) and ((.body // "") | length > 0))
      | [ .id, $pr.number, $pr.headRefName, "review-\(.state | ascii_downcase)",
          "-", "-", $author, .url ] )
  )
| @tsv
