# Turns the output of `forge.sh list-threads` into one TSV row per comment
# worth acting on:
#   note id, thread id, location, author
#
# Kept: comments in threads that are not resolved, not seen yet.
# Dropped: comments by $me (the flow's own replies, and in version 1 the
# developer's own comments, since both use the same account) and ids in $seen.
# Every field is non-empty, because bash `read` merges empty TSV fields.

.[]
| select(.resolved | not)
| . as $thread
| .notes[]
| select(.author != $me)
| select(.id as $id | $seen | index($id) | not)
| [ .id, $thread.thread,
    (if $thread.path then "\($thread.path):\($thread.line // "outdated")" else "general" end),
    (.author // "ghost") ]
| @tsv
