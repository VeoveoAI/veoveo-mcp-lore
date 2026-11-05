// The Lore repositories a person can read, then their folders and files. A location is
// { repository: { id, name }, folders: [names] }, and none is the list of repositories.
veoveoGallery((ui) => {
  const BRANCH = "main";
  const pathOf = (folders, name) => [...folders, name].join("/");
  const viewUrl = (repository, path) =>
    `/${["view", "lore", repository, BRANCH, ...path.split("/")].map(encodeURIComponent).join("/")}`;
  const listed = async (repository, folders) => {
    const args = { repository: repository.id, branch: BRANCH };
    if (folders.length) args.path = folders.join("/");
    return (await ui.tool("list_files", args)).entries || [];
  };

  const repositoryItem = (repository) => ({
    key: `repository\0${repository.id}`,
    name: repository.name,
    kind: "repository",
    about: "Repository",
    folder: true,
    repository,
    folders: [],
  });
  const itemOf = (repository, folders, entry) => {
    const path = pathOf(folders, entry.name);
    const key = `${repository.id}\0${path}`;
    if (entry.type === "directory") {
      return { key, name: entry.name, kind: "folder", about: ui.size(entry.size) || "Folder", folder: true, repository, folders: [...folders, entry.name] };
    }
    if (entry.type === "link") {
      return { key, name: entry.name, kind: "link", about: "Linked repository", disabled: true };
    }
    const kind = ui.kindOf(entry.name);
    return {
      key,
      name: entry.name,
      kind,
      about: [ui.size(entry.size), ui.labelOf(kind)].filter(Boolean).join(" · "),
      detail: [ui.labelOf(kind), ui.size(entry.size), `${repository.name}/${path}`].filter(Boolean).join(" · "),
      repository,
      path,
    };
  };

  const trailOf = (at) => {
    const trail = [{ label: "Repositories", at: null }, { label: at.repository.name, at: { repository: at.repository, folders: [] } }];
    at.folders.forEach((folder, index) => {
      trail.push({ label: folder, at: { repository: at.repository, folders: at.folders.slice(0, index + 1) } });
    });
    return trail;
  };

  // Pictures for a folder's card: what it holds first, then a file from each folder inside.
  const RANK = { image: 0, video: 0, scene: 0, model: 1, document: 2, audio: 3 };
  const usable = (repository, folders, entries) => entries
    .map((entry) => itemOf(repository, folders, entry))
    .filter((item) => item.path && item.kind in RANK)
    .sort((a, b) => RANK[a.kind] - RANK[b.kind]);

  return {
    title: "Repositories",
    list: async ({ at }) => {
      if (!at) {
        const listing = await ui.tool("list_files", {});
        return {
          items: (listing.repositories || []).map(repositoryItem),
          trail: [{ label: "Repositories" }],
          empty: "You can read no repositories yet.",
        };
      }
      const entries = await listed(at.repository, at.folders);
      return {
        items: entries.map((entry) => itemOf(at.repository, at.folders, entry)),
        trail: trailOf(at),
        empty: "This folder is empty.",
      };
    },
    enter: (item) => ({ repository: item.repository, folders: item.folders }),
    cover: async (item) => {
      const entries = await listed(item.repository, item.folders);
      const found = usable(item.repository, item.folders, entries);
      for (const entry of entries.filter((one) => one.type === "directory").slice(0, 6)) {
        if (found.length >= 4) break;
        const folders = [...item.folders, entry.name];
        const first = usable(item.repository, folders, await listed(item.repository, folders))[0];
        if (first) found.push(first);
      }
      return found;
    },
    thumbnail: (item, size) => ui.tool("thumbnail", { repository: item.repository.id, branch: BRANCH, path: item.path, size }, ui.PICTURE_TIMEOUT),
    open: (item) => viewUrl(item.repository.id, item.path),
  };
});
