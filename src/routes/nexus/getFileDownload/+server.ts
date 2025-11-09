import Nexus from "@nexusmods/nexus-api";

const NexusKey = import.meta.env.PROD_NEXUS_KEY
const NEXUS = await Nexus.create(NexusKey, "Wildcard", "0.1", "balatro")

export async function GET({ url }) {
    let modID: number = parseInt(url.searchParams.get("modID"))
    let file: number = parseInt(url.searchParams.get("fileID"))

    return new Response("41")
}