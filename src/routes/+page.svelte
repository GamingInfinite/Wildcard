<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Octokit } from "octokit";
  import { onMount } from "svelte";
  import MdCheck from "svelte-icons/md/MdCheck.svelte";
  import MdSearch from "svelte-icons/md/MdSearch.svelte";

  import modLinks from "$lib/modlinks.txt?raw";
  import modLinksAdv from "$lib/modlinksAdvanced.json";
  import { appDataDir, BaseDirectory } from "@tauri-apps/api/path";

  let ModDirectory = "";

  let GITHUB_KEY = import.meta.env.PROD_GITHUB_KEY;

  let octokit: Octokit;

  type RepoTags = {
    owner: string;
    repo: string;
    displayName: string;
    star?: boolean;
    tags: { tag: string; sha: string; date?: string }[];
  };

  type CombinedModInfo = {
    name: string;
    star?: boolean;
    url?: string;
    repoInfo?: RepoTags;
  };

  let releaseTagsByMod: Map<string, RepoTags> = new Map();

  let modSelects: { [string: string]: string } = {};

  let modLinkList = modLinks
    .replaceAll("https://github.com/", "")
    .split("\r\n");

  let modLinkAdvList: { name: string; url: string; star?: boolean }[] =
    modLinksAdv;

  $: combinedModList = combineModInfo(releaseTagsByMod, modLinkAdvList);

  let modListSearchInput = "";
  let modListSearch: CombinedModInfo[] = [];

  function combineModInfo(...args: any[]) {
    let combinedModList: CombinedModInfo[] = [];
    releaseTagsByMod.forEach((tags, id) => {
      let modInfo: CombinedModInfo = {
        name: id,
        star: tags.star,
        repoInfo: tags,
      };
      combinedModList.push(modInfo);
    });
    combinedModList = combinedModList.concat(modLinkAdvList);

    combinedModList = combinedModList.sort((a, b) => {
      // First, prioritize starred mods
      if (a.star && !b.star) return -1;
      if (!a.star && b.star) return 1;

      // If both have same star status, sort alphabetically by name
      let aname: string, bname: string;
      if (a.repoInfo) {
        aname = a.repoInfo.displayName;
      } else {
        aname = a.name;
      }

      if (b.repoInfo) {
        bname = b.repoInfo.displayName;
      } else {
        bname = b.name;
      }
      return aname.localeCompare(bname);
    });

    modListSearch = combinedModList;

    return combinedModList;
  }

  let modPull: string[] = [];
  let pullPromises: boolean[] = [];

  async function clone(owner: string, repo: string, sha?: string) {
    let link = `https://github.com/${owner}/${repo}`;

    try {
      let response;
      if (sha) {
        response = await invoke("clone_repo", {
          repoUrl: link,
          destination:
            ModDirectory + `/${releaseTagsByMod.get(repo)?.displayName}`,
          commitSha: sha,
        });
      } else {
        response = await invoke("clone_repo", {
          repoUrl: link,
          destination:
            ModDirectory + `/${releaseTagsByMod.get(repo)?.displayName}`,
        });
      }
      console.log(response);
    } catch (err) {
      console.log(err);
    }
  }

  async function advClone(repo: string, url: string) {
    console.log(repo);
    try {
      let response = await invoke("extract_folder_from_repo", {
        repoUrl: url,
        destinationPath: ModDirectory + `/${repo}`,
        folderInRepo: repo,
      });
      console.log(response);
    } catch (err) {
      console.log(err);
    }
  }

  function TagBySHA(
    repo: string,
    sha: string
  ): { tag: string; sha: string; date?: string } {
    let tags = releaseTagsByMod.get(repo)?.tags;
    if (tags) {
      for (let i = 0; i < tags.length; i++) {
        if (tags[i].sha == sha) {
          return tags[i];
        }
      }
    }
    return { tag: "", sha: "", date: "" };
  }

  async function getReleaseTags(
    owner: string,
    repo: string,
    disp_name?: string,
    star = false
  ) {
    let repoTags: RepoTags = { owner, repo, tags: [], displayName: repo, star };
    if (disp_name) {
      repoTags.displayName = disp_name;
    }
    await octokit
      .request(`GET /repos/${owner}/${repo}/tags?per_page=100`, {
        owner: "OWNER",
        repo: "REPO",
        headers: {
          "X-GitHub-Api-Version": "2022-11-28",
        },
      })
      .then((res) => {
        Object.values(res.data).forEach((tagData) => {
          type tempTag = {
            name: string;
            commit: { sha: string };
          };
          let tagform = tagData as tempTag;

          repoTags.tags.push({ tag: tagform.name, sha: tagform.commit.sha });
        });

        releaseTagsByMod.set(repo, repoTags);

        releaseTagsByMod = releaseTagsByMod;
      });

    let tags = repoTags.tags.map((item) => item.sha);

    let date = new Date();
    date.setMonth(date.getMonth() - 3);
    await octokit
      .request(
        `GET /repos/${owner}/${repo}/commits?per_page=100&since=${date.toISOString()}`,
        {
          owner: "OWNER",
          repo: "REPO",
          headers: {
            "X-GitHub-Api-Version": "2022-11-28",
          },
        }
      )
      .then((res) => {
        type SHAFilter = {
          sha: string;
        };
        let TagsFiltered: RepoTags = {
          owner,
          repo,
          tags: [],
          displayName: repoTags.displayName,
        };
        Object.values(res.data).forEach((commitData) => {
          let commitSHA = commitData as SHAFilter;
          if (tags?.includes(commitSHA.sha)) {
            TagsFiltered.tags.push(TagBySHA(repo, commitSHA.sha));
          }
        });
        releaseTagsByMod.set(repo, TagsFiltered);
      });
  }

  async function runPull() {
    await invoke("nuke_directory", { path: ModDirectory });
    pullPromises = [];
    for (let i = 0; i < modPull.length; i++) {
      let repo_sha = modPull[i].split("-");
      let owner = releaseTagsByMod.get(repo_sha[0])?.owner;

      let promise: Promise<void>;

      if (owner) {
        if (repo_sha[1] == "latest") {
          promise = clone(owner, repo_sha[0]);
        } else {
          promise = clone(owner, repo_sha[0], repo_sha[1]);
        }
      } else {
        let modName = repo_sha[1];
        let modURL: string = "";
        for (let i = 0; i < modLinkAdvList.length; i++) {
          let mod = modLinkAdvList[i];
          if (mod.name.includes(modName)) {
            modURL = mod.url;
            modName = mod.name;
          }
        }
        promise = advClone(modName, modURL);
      }

      pullPromises[i] = false;
      promise.finally(() => {
        pullPromises[i] = true;
      });
    }
  }

  onMount(async () => {
    let path: string = await appDataDir()
    path = path.replaceAll("com.wildcard.app", "")
    path += "Balatro\\Mods"

    ModDirectory = path

    octokit = new Octokit({ auth: GITHUB_KEY });

    modLinkList.forEach(async (modlink) => {
      let star = modlink.slice(0, 1) == "*";
      modlink = modlink.replaceAll("*", "");
      let disp_name = modlink.split("#");
      let ownrepo = modlink.split("/");
      if (disp_name[1]) {
        ownrepo = disp_name[0].split("/");
      }
      getReleaseTags(ownrepo[0], ownrepo[1], disp_name[1], star);
      modSelects[ownrepo[1]] = "latest";
    });

    releaseTagsByMod = releaseTagsByMod;
  });
</script>

<div class="flex flex-row justify-center m-4 gap-4">
  <div class="flex flex-col gap-2 w-full">
    <label class="input">
      <div class="w-8">
        <MdSearch />
      </div>
      <input
        type="search"
        class="grow"
        placeholder="Search"
        bind:value={modListSearchInput}
        on:input={() => {
          modListSearch = [];
          for (let i = 0; i < combinedModList.length; i++) {
            let mod = combinedModList[i];
            let dispName = "";
            if (mod.repoInfo) {
              dispName = mod.repoInfo.displayName;
            } else {
              dispName = mod.name;
            }
            dispName = dispName.toLocaleLowerCase();

            if (dispName.search(modListSearchInput.toLocaleLowerCase()) != -1) {
              modListSearch.push(mod);
            }
          }
        }}
      />
    </label>

    <div class="grid grid-cols-3 gap-4">
      {#each modListSearch as { name, url, star, repoInfo }, i}
        <div class="card bg-base-200 card-sm shadow-sm">
          <div class="card-body">
            <h2 class="card-title">
              {#if repoInfo}
                {repoInfo.displayName}
              {:else}
                {name}
              {/if}
              {#if star}
                ⭐
              {/if}
            </h2>
            <div class="flex flex-col gap-2">
              <select class="select" bind:value={modSelects[name]}>
                <option value={`latest`} selected>Latest</option>
                {#if repoInfo}
                  {#each repoInfo.tags as tag}
                    <option value={`${tag.sha}`}
                      >{tag.tag} - {tag.sha.substring(0, 6)}</option
                    >
                  {/each}
                {/if}
              </select>
            </div>
            <div class="justify-end card-actions">
              <button
                class="btn btn-primary"
                on:click={() => {
                  for (let i = 0; i < modPull.length; i++) {
                    let modName = modPull[i].substring(
                      0,
                      modPull[i].lastIndexOf("-")
                    );
                    if (repoInfo) {
                      if (modName == repoInfo.repo) {
                        modPull.splice(i, 1);
                        i--;
                      }
                    } else {
                      if (modPull[i].includes(name)) {
                        modPull.splice(i, 1);
                        i--;
                      }
                    }
                  }
                  modPull.push(`${name}-${modSelects[name]}`);
                  modPull = modPull;
                }}>Add Mod</button
              >
            </div>
          </div>
        </div>
      {/each}
    </div>
  </div>
  <div class="flex flex-col gap-2 w-1/2">
    <div class="flex flex-row font-bold">Mod List</div>
    <div class="flex flex-col min-w-64">
      {#each modPull as pull, i}
        <div class="flex flex-row gap-2 items-center">
          <div>
            {pull}
          </div>
          {#if pullPromises[i]}
            <div class="w-6">
              <MdCheck />
            </div>
          {:else}
            <span class="loading loading-dots loading-md"></span>
          {/if}
        </div>
      {/each}
    </div>
    <div class="flex flex-row">
      <button
        class="btn btn-primary min-w-32"
        on:click={runPull}
        disabled={!pullPromises.every((value) => value === true)}
      >
        {#if !pullPromises.every((value) => value === true)}
          <span class="loading loading-spinner loading-md"></span>
        {:else}
          Download
        {/if}
      </button>
    </div>
  </div>
</div>
