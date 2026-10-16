import { BasePage } from "@/app/base-page";
import SharedProjectPage from "./share-client";

export default async function SharePage({ params }: { params: Promise<{ token: string }> }) {
	const { token } = await params;
	return (
		<BasePage title="Shared OpenCut project" description="Read-only project share">
			<SharedProjectPage token={token} />
		</BasePage>
	);
}
