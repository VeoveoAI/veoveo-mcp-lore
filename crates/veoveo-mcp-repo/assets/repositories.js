// The Lore repositories a person can read, then what each holds, a page at a time. The server says
// what shows each folder, which folders are one object, and answers the pictures that are ready with
// the page. A location is { repository: { id, name }, folders: [names] }, and none is the list of
// repositories.
veoveoGallery((ui) => {
  veoveoViewer.moves(ui.go);
  const branch = () => veoveoViewer.filesBranch();
  const viewUrl = (repository, path) =>
    `/${["view", "lore", repository, branch(), ...path.split("/")].map(encodeURIComponent).join("/")}`;
  const keyOf = (repository, path) => `${repository.id}\0${path}`;
  const coverOf = (repository, cover) => cover.map((path) => ({ key: keyOf(repository, path), repository, path }));

  const repositoryItem = (entry) => {
    const repository = { id: entry.repository, name: entry.name };
    return {
      key: `repository\0${repository.id}`,
      name: repository.name,
      kind: "repository",
      about: "Repository",
      folder: true,
      repository,
      folders: [],
      cover: entry.cover && coverOf(repository, entry.cover),
    };
  };
  const itemOf = (repository, entry) => {
    if (entry.type === "link") {
      return { key: keyOf(repository, entry.path), name: entry.name, kind: "link", about: "Linked repository", disabled: true };
    }
    const fields = { repository, branch: branch(), path: entry.stage || entry.path, type: entry.type, size: entry.size };
    if (entry.stage) {
      return {
        key: keyOf(repository, entry.stage),
        name: entry.name,
        kind: "scene",
        badge: "",
        about: ui.size(entry.size),
        fields,
        repository,
        path: entry.stage,
        into: { repository, folders: entry.path.split("/") },
      };
    }
    if (entry.type === "directory") {
      return {
        key: keyOf(repository, entry.path),
        name: entry.name,
        kind: "folder",
        about: ui.size(entry.size) || "Folder",
        folder: true,
        repository,
        folders: entry.path.split("/"),
        cover: entry.cover && coverOf(repository, entry.cover),
      };
    }
    return {
      key: keyOf(repository, entry.path),
      name: entry.name,
      kind: ui.kindOf(entry.name),
      about: ui.size(entry.size),
      fields,
      repository,
      path: entry.path,
    };
  };

  // While the server is drawing a repository's pictures ahead, an open page keeps it company, since
  // the drawing lasts only as long as someone's last request. A hidden page does not.
  let company = null;
  const keepCompany = (at, filling) => {
    clearTimeout(company);
    if (!at || !filling) return;
    company = setTimeout(async () => {
      if (document.visibilityState !== "visible") return keepCompany(at, true);
      const page = await ui.tool("browse", { repository: at.repository.id, branch: branch(), limit: 1 }).catch(() => ({ filling: true }));
      keepCompany(at, page.filling);
    }, 30000);
  };

  const trailOf = (at) => {
    const trail = [{ label: "Repositories", at: null }, { label: at.repository.name, at: { repository: at.repository, folders: [] } }];
    at.folders.forEach((folder, index) => {
      trail.push({ label: folder, at: { repository: at.repository, folders: at.folders.slice(0, index + 1) } });
    });
    return trail;
  };

  return {
    title: "Repo",
    placeholder: "Search",
    remoteSearch: true,
    list: async ({ at, search, cursor }) => {
      veoveoViewer.inRepository(at ? at.repository : null);
      const asked = {};
      if (at) {
        asked.repository = at.repository.id;
        asked.branch = branch();
        if (at.folders.length) asked.path = at.folders.join("/");
      }
      if (search) asked.search = search;
      if (cursor) asked.cursor = cursor;
      const page = await ui.tool("browse", asked);
      keepCompany(at, page.filling);
      return {
        items: at ? page.items.map((entry) => itemOf(at.repository, entry)) : page.items.map(repositoryItem),
        next: page.next,
        trail: at ? trailOf(at) : [{ label: "Repositories" }],
        empty: at ? "Nothing here." : "You can read no repositories yet.",
        pictures: page.pictures.map((picture) => ({
          key: `${picture.repository}\0${picture.path}`,
          mediaType: picture.mediaType,
          data: picture.data,
        })),
      };
    },
    enter: (item) => ({ repository: item.repository, folders: item.folders }),
    covers: async (items) => {
      const { covers } = await ui.tool("covers", {
        branch: branch(),
        folders: items.map((item) => (item.folders.length
          ? { repository: item.repository.id, path: item.folders.join("/") }
          : { repository: item.repository.id })),
      });
      return items.map((item, index) => coverOf(item.repository, covers[index]?.files || []));
    },
    thumbnails: async (items, size) => (await ui.tool("thumbnails", {
      files: items.map((item) => ({ repository: item.repository.id, branch: branch(), path: item.path })),
      size,
    }, ui.PICTURE_TIMEOUT)).thumbnails,
    thumbnail: (item, size) => ui.tool("thumbnail", { repository: item.repository.id, branch: branch(), path: item.path, size }, ui.PICTURE_TIMEOUT),
    open: (item) => viewUrl(item.repository.id, item.path),
  };
});
