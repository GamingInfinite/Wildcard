import Nexus from "@nexusmods/nexus-api";

const NexusKey = import.meta.env.PROD_NEXUS_KEY

type ModFileTruncate = {
    file_id: number
    name: string
}

const NEXUS = await Nexus.create(NexusKey, "Wildcard", "0.1", "balatro")

export async function GET({ url }) {
    let modID = parseInt(url.searchParams.get("modID"))
    let modFilesInfo = (await NEXUS.getModFiles(modID)).files

    let modFilesTrunc: ModFileTruncate[] = []

    for (let i = 0; i < modFilesInfo.length; i++) {
        let modFile: ModFileTruncate = {
            file_id: modFilesInfo[i].file_id,
            name: modFilesInfo[i].name
        }

        modFilesTrunc.push(modFile)
    }

    return new Response(JSON.stringify(modFilesTrunc))
}