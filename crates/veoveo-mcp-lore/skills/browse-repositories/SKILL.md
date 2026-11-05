---
name: browse-repositories
description: See what the Lore repositories you can read hold, and give a person a link that opens a file or shows them the gallery. Use when someone asks what is in a repository or wants to look through its files.
---

# Browse repositories

Every read is yours: a repository you hold no grant on is refused.

1. Call `lore__list_files` with nothing to list the repositories you can read, each with its `id` and `name`, then with that
   `id` as `repository`, and a `branch` if it is not `main`, to list its top. Give a folder's path as `path` to list it.
2. To show a person, name the App: `/apps/lore/repositories` on the Hub, a gallery with a picture of each file.
3. To open one file for a person, give the Hub's link: `/view/lore/{repository}/{branch}/{path}`, with each path segment
   percent-encoded. It opens with the right viewer, for whoever follows it and may read the file.
