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
    return {
      key,
      name: entry.name,
      kind: ui.kindOf(entry.name),
      about: ui.size(entry.size),
      fields: { repository, branch: BRANCH, path, type: entry.type, size: entry.size },
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
    // The server says what shows each folder.
    covers: async (items) => {
      const { covers } = await ui.tool("covers", {
        branch: BRANCH,
        folders: items.map((item) => (item.folders.length
          ? { repository: item.repository.id, path: item.folders.join("/") }
          : { repository: item.repository.id })),
      });
      return items.map((item, index) => (covers[index]?.files || []).map((path) => ({
        key: `${item.repository.id}\0${path}`,
        repository: item.repository,
        path,
      })));
    },
    thumbnails: async (items, size) => (await ui.tool("thumbnails", {
      files: items.map((item) => ({ repository: item.repository.id, branch: BRANCH, path: item.path })),
      size,
    }, ui.PICTURE_TIMEOUT)).thumbnails,
    thumbnail: (item, size) => ui.tool("thumbnail", { repository: item.repository.id, branch: BRANCH, path: item.path, size }, ui.PICTURE_TIMEOUT),
    open: (item) => viewUrl(item.repository.id, item.path),
  };
});
