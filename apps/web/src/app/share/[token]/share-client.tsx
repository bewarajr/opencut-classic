"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";

interface SharedAsset {
	id: string;
	name: string;
	type: "video" | "audio" | "image";
	mimeType?: string;
}

interface SharedProject {
	metadata: { name: string; thumbnail?: string; duration?: number };
	scenes?: Array<{ id: string; name: string }>;
}

export default function SharedProjectPage({ token }: { token: string }) {
	const [project, setProject] = useState<SharedProject | null>(null);
	const [assets, setAssets] = useState<SharedAsset[]>([]);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		let cancelled = false;
		void (async () => {
			try {
				const response = await fetch(`/api/cloud/share/${encodeURIComponent(token)}`, { cache: "no-store" });
				const payload = (await response.json()) as { project?: SharedProject; assets?: SharedAsset[]; error?: string };
				if (!response.ok || !payload.project) throw new Error(payload.error ?? "This share link is unavailable");
				if (!cancelled) {
					setProject(payload.project);
					setAssets(payload.assets ?? []);
				}
			} catch (cause) {
				if (!cancelled) setError(cause instanceof Error ? cause.message : "This share link is unavailable");
			}
		})();
		return () => { cancelled = true; };
	}, [token]);

	return (
		<>
			{error ? (
				<div className="mx-auto max-w-xl rounded-xl border p-6 text-center">
					<p>{error}</p>
					<Button asChild className="mt-4"><Link href="/projects">Open OpenCut</Link></Button>
				</div>
			) : !project ? (
				<p className="text-center text-muted-foreground">Loading shared project…</p>
			) : (
				<div className="mx-auto flex w-full max-w-3xl flex-col gap-6">
					<h2 className="text-center text-2xl font-semibold">{project.metadata.name}</h2>
					<div className="rounded-xl border p-5">
						<p className="text-sm text-muted-foreground">Shared read-only · {project.scenes?.length ?? 0} scene(s) · {assets.length} media file(s)</p>
						{project.metadata.thumbnail && <img className="mt-4 max-h-80 w-full rounded-lg object-contain" src={project.metadata.thumbnail} alt="Project thumbnail" />}
					</div>
					{assets.length === 0 ? (
						<p className="rounded-xl border p-5 text-sm text-muted-foreground">This project has no uploaded media assets.</p>
					) : assets.map((asset) => {
						const src = `/api/cloud/share/${encodeURIComponent(token)}/media/${encodeURIComponent(asset.id)}`;
						return (
							<div key={asset.id} className="rounded-xl border p-4">
								<p className="mb-3 truncate text-sm font-medium">{asset.name}</p>
								{asset.type === "video" ? <video className="max-h-[70vh] w-full rounded-lg bg-black" controls preload="metadata" src={src} /> : asset.type === "audio" ? <audio className="w-full" controls preload="metadata" src={src} /> : <img className="max-h-[70vh] w-full rounded-lg object-contain" src={src} alt={asset.name} />}
							</div>
						);
					})}
				</div>
			)}
		</>
	);
}
