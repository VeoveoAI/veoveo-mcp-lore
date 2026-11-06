---
name: browse-repositories
description: See what people and agents did in the Lore repositories you can read and what those repositories hold, and give a person a link that opens a file or shows them the viewer. Use when someone asks what is in a repository or wants to look through its files.
---

# Browse repositories

Every read is yours: a repository you hold no grant on is refused.

1. Call `repo__list_files` with nothing to list the repositories you can read, each with its `id` and `name`, then with that
   `id` as `repository`, and a `branch` if it is not `main`, to list its top. Give a folder's path as `path` to list it.
2. To show a person, name the App: `/apps/repo/activity` on the Hub, which opens on what was pushed lately, with pictures, diffs and whether it is running, and has a gallery of each repository's files.
3. To open one file for a person, give the Hub's link: `/view/lore/{repository}/{branch}/{path}`, with each path segment
   percent-encoded. It opens with the right viewer, for whoever follows it and may read the file.
4. To see what was done, call `repo__activity`, with a `repository` and a `branch` to narrow it, then `repo__changes` for what one revision changed and `repo__file_diff` for a text file's diff. To read a file as it was at a revision, give `@{revision}` as the branch, in a tool or in the link above.
