"use client";

import Link from "next/link";
import { useEffect, useState, type FormEvent } from "react";
import { toast } from "sonner";
import { signIn, signOut, signUp, useSession } from "@/auth/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { storageService } from "@/services/storage/service";

interface ApiToken {
	id: string;
	createdAt: string;
}

export default function AccountPage() {
	const { data: session, isPending } = useSession();
	const [mode, setMode] = useState<"sign-in" | "sign-up">("sign-in");
	const [name, setName] = useState("");
	const [email, setEmail] = useState("");
	const [password, setPassword] = useState("");
	const [isSubmitting, setIsSubmitting] = useState(false);
	const [tokens, setTokens] = useState<ApiToken[]>([]);
	const [newToken, setNewToken] = useState<string | null>(null);
	const [isLoadingTokens, setIsLoadingTokens] = useState(false);

	const loadTokens = async () => {
		setIsLoadingTokens(true);
		try {
			const response = await fetch("/api/cloud/tokens", { cache: "no-store" });
			if (!response.ok) throw new Error("Could not load MCP tokens");
			const payload = (await response.json()) as { tokens: ApiToken[] };
			setTokens(payload.tokens);
		} catch (error) {
			toast.error(error instanceof Error ? error.message : "Could not load MCP tokens");
		} finally {
			setIsLoadingTokens(false);
		}
	};

	useEffect(() => {
		if (session?.user) void loadTokens();
	}, [session?.user]);

	const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		setIsSubmitting(true);
		const isCreatingAccount = mode === "sign-up";
		try {
			const result =
				mode === "sign-up"
					? await signUp.email({ name: name.trim(), email: email.trim(), password })
					: await signIn.email({ email: email.trim(), password });
			if (result.error) throw new Error(result.error.message ?? "Authentication failed");
		} catch (error) {
			toast.error(error instanceof Error ? error.message : "Authentication failed");
			setIsSubmitting(false);
			return;
		}
		if (isCreatingAccount) {
			try {
				const sync = await storageService.syncLocalProjectsToCloud();
				if (sync.failed > 0) {
					toast.warning(`Account ready. ${sync.failed} project or media item could not be synced.`);
				} else {
					toast.success(`Account ready. ${sync.uploaded} local project(s) synced.`);
				}
			} catch (error) {
				toast.warning(error instanceof Error ? error.message : "Account is ready, but local project sync failed");
			}
		} else {
			toast.success("Signed in to your cloud account.");
		}
		setIsSubmitting(false);
		window.location.assign("/projects");
	};

	const importLocalProjects = async () => {
		try {
			const result = await storageService.syncLocalProjectsToCloud();
			if (result.failed > 0) toast.warning(`${result.uploaded} project(s) synced; ${result.failed} item(s) need attention.`);
			else toast.success(`${result.uploaded} local project(s) synced to this account.`);
		} catch (error) {
			toast.error(error instanceof Error ? error.message : "Could not sync local projects");
		}
	};

	const createToken = async () => {
		try {
			const response = await fetch("/api/cloud/tokens", { method: "POST" });
			const payload = (await response.json()) as { token?: string; error?: string };
			if (!response.ok || !payload.token) throw new Error(payload.error ?? "Could not create token");
			setNewToken(payload.token);
			await loadTokens();
		} catch (error) {
			toast.error(error instanceof Error ? error.message : "Could not create token");
		}
	};

	const revokeToken = async (id: string) => {
		try {
			const response = await fetch(`/api/cloud/tokens/${encodeURIComponent(id)}`, { method: "DELETE" });
			if (!response.ok) throw new Error("Could not revoke token");
			setTokens((current) => current.filter((token) => token.id !== id));
			toast.success("MCP token revoked");
		} catch (error) {
			toast.error(error instanceof Error ? error.message : "Could not revoke token");
		}
	};

	return (
		<div className="mx-auto flex w-full max-w-xl flex-col gap-8">
			{isPending ? (
				<p className="text-muted-foreground text-center">Loading account…</p>
			) : session?.user ? (
				<div className="mx-auto flex w-full max-w-xl flex-col gap-8">
					<section className="rounded-xl border p-6">
						<p className="text-sm text-muted-foreground">Signed in as</p>
						<p className="mt-1 font-medium">{session.user.name}</p>
						<p className="text-sm text-muted-foreground">{session.user.email}</p>
						<div className="mt-5 flex gap-3">
							<Button asChild><Link href="/projects">Open projects</Link></Button>
							<Button variant="outline" onClick={() => void signOut()}>Sign out</Button>
						</div>
						<Button className="mt-4" variant="outline" onClick={() => void importLocalProjects()}>Sync local projects to cloud</Button>
					</section>

					<section className="rounded-xl border p-6">
						<h2 className="text-lg font-semibold">Connect OpenCut MCP</h2>
						<p className="mt-2 text-sm text-muted-foreground">Create a token so projects created by MCP are saved to this account. The token is shown only once.</p>
						{newToken && (
							<div className="mt-4 rounded-lg bg-muted p-3">
								<p className="mb-2 text-xs text-muted-foreground">Copy this token into the MCP controller environment as OPENCUT_API_TOKEN.</p>
								<code className="block break-all text-xs">{newToken}</code>
								<Button className="mt-3" size="sm" variant="outline" onClick={() => void navigator.clipboard.writeText(newToken).then(() => toast.success("Token copied"))}>Copy token</Button>
							</div>
						)}
						<Button className="mt-4" onClick={() => void createToken()}>Create MCP token</Button>
						<div className="mt-5 flex flex-col gap-2">
							{isLoadingTokens ? <p className="text-sm text-muted-foreground">Loading tokens…</p> : tokens.map((token) => (
								<div key={token.id} className="flex items-center justify-between rounded-lg border px-3 py-2">
									<span className="text-sm">MCP token · {new Date(token.createdAt).toLocaleDateString()}</span>
									<Button variant="ghost" size="sm" onClick={() => void revokeToken(token.id)}>Revoke</Button>
								</div>
							))}
						</div>
					</section>
				</div>
			) : (
				<form onSubmit={handleSubmit} className="mx-auto flex w-full max-w-md flex-col gap-4 rounded-xl border p-6">
					<div className="flex gap-2">
						<Button type="button" variant={mode === "sign-in" ? "default" : "outline"} onClick={() => setMode("sign-in")}>Sign in</Button>
						<Button type="button" variant={mode === "sign-up" ? "default" : "outline"} onClick={() => setMode("sign-up")}>Create account</Button>
					</div>
					{mode === "sign-up" && <Input autoComplete="name" required placeholder="Name" value={name} onChange={(event) => setName(event.target.value)} />}
					<Input autoComplete="email" type="email" required placeholder="Email" value={email} onChange={(event) => setEmail(event.target.value)} />
					<Input autoComplete={mode === "sign-up" ? "new-password" : "current-password"} type="password" minLength={8} required placeholder="Password (at least 8 characters)" value={password} onChange={(event) => setPassword(event.target.value)} />
					<Button type="submit" disabled={isSubmitting}>{isSubmitting ? "Working…" : mode === "sign-up" ? "Create cloud account" : "Sign in"}</Button>
					<p className="text-xs text-muted-foreground">Projects and media you sync are stored on the Bewara server under this account.</p>
				</form>
			)}
		</div>
	);
}
