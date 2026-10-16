import { BasePage } from "@/app/base-page";
import AccountClient from "./account-client";

export default function AccountPage() {
	return (
		<BasePage
			title="Cloud account"
			description="Sign in once to keep your projects and media available across browsers and devices."
		>
			<AccountClient />
		</BasePage>
	);
}
