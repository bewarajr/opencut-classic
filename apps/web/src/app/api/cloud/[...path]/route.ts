import { auth } from "@/auth/server";
import { webEnv } from "@/env/web";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

type RouteContext = { params: Promise<{ path: string[] }> };

async function proxyCloudRequest(request: Request, context: RouteContext) {
	if (!webEnv.CLOUD_API_URL) {
		return Response.json({ error: "Cloud project storage is not configured" }, { status: 503 });
	}
	if (!webEnv.PROJECTS_INTERNAL_SECRET) {
		return Response.json({ error: "Cloud project storage is not configured" }, { status: 503 });
	}

	const { path } = await context.params;
	const cloudPath = path.map(encodeURIComponent).join("/");
	const targetUrl = new URL(`${webEnv.CLOUD_API_URL.replace(/\/$/, "")}/${cloudPath}`);
	targetUrl.search = new URL(request.url).search;

	const headers = new Headers();
	for (const key of [
		"accept",
		"authorization",
		"content-length",
		"content-type",
		"x-opencut-media-meta",
	]) {
		const value = request.headers.get(key);
		if (value) headers.set(key, value);
	}

	const isPublicShareRead = path[0] === "share" && request.method === "GET";
	if (!isPublicShareRead) {
		const session = await auth.api.getSession({ headers: request.headers });
		if (session?.user?.id) {
			headers.set("x-opencut-user-id", session.user.id);
			headers.set("x-opencut-internal-secret", webEnv.PROJECTS_INTERNAL_SECRET);
		}
	}

	const hasBody = request.method !== "GET" && request.method !== "HEAD";
	const init = {
		method: request.method,
		headers,
		...(hasBody ? { body: request.body, duplex: "half" as const } : {}),
		cache: "no-store" as const,
		redirect: "manual" as const,
	} as RequestInit;
	const response = await fetch(targetUrl, init);
	const responseHeaders = new Headers(response.headers);
	responseHeaders.delete("transfer-encoding");
	responseHeaders.delete("connection");
	return new Response(response.body, {
		status: response.status,
		statusText: response.statusText,
		headers: responseHeaders,
	});
}

export const GET = proxyCloudRequest;
export const POST = proxyCloudRequest;
export const PUT = proxyCloudRequest;
export const DELETE = proxyCloudRequest;
