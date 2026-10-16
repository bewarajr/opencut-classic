import type { Metadata } from "next";
import { BasePage } from "@/app/base-page";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Separator } from "@/components/ui/separator";
import { SOCIAL_LINKS } from "@/site/social";

export const metadata: Metadata = {
	title: "Privacy Policy - OpenCut",
	description:
		"Learn how OpenCut handles your data and privacy. Our commitment to protecting your information while you edit videos.",
	openGraph: {
		title: "Privacy Policy - OpenCut",
		description:
			"Learn how OpenCut handles your data and privacy. Our commitment to protecting your information while you edit videos.",
		type: "website",
	},
};

export default function PrivacyPage() {
	return (
		<BasePage
			title="Privacy policy"
			description="Learn how we handle your data and privacy. Contact us if you have any questions."
		>
			<Accordion type="single" collapsible className="w-full">
				<AccordionItem
					value="quick-summary"
					className="rounded-2xl border px-5"
				>
					<AccordionTrigger className="no-underline!">
						Quick summary
					</AccordionTrigger>
					<AccordionContent>
						<h3 className="mb-3 text-lg font-medium">
							Your content stays local unless you choose cloud sync.
						</h3>
						<ol className="list-decimal space-y-2 pl-6">
							<li>
								Basic editing happens locally. Synced projects and media are stored
								on the Bewara server for access across devices.
							</li>
							<li>
								AI features like auto captions run locally in your browser.
							</li>
							<li>
								Cloud sync requires an optional email and password account.
							</li>
							<li>Un-synced project data stays on your device</li>
							<li>Share links are read-only and available to anyone who has the link</li>
							<li>
								We use anonymized analytics to improve the app, but no personal video
								content is tracked
							</li>
							<li>You can clear local data from your browser at any time</li>
							<li>
								We don&apos;t sell project or media data.
							</li>
						</ol>
						<p className="mt-4">
							Questions? Email us at{" "}
							<a
								href="mailto:oss@opencut.app"
								className="text-primary hover:underline"
							>
								oss@opencut.app
							</a>
						</p>
					</AccordionContent>
				</AccordionItem>
			</Accordion>

			<section className="flex flex-col gap-3">
				<h2 className="text-2xl font-semibold">How We Handle Your Content</h2>
				<p>
					<strong>Editing happens locally in your browser.</strong>{" "}
					If you sign in and sync a project, its timeline and media files are
					uploaded to the Bewara server so you can open them on other devices.
					Projects stay private to your account unless you create a share link.
					Anyone with an active read-only share link can view the project and its
					uploaded media until you revoke that link. AI captions still run locally.
				</p>
			</section>

			<section className="flex flex-col gap-3">
				<h2 className="text-2xl font-semibold">Accounts & Authentication</h2>
				<p>
					OpenCut offers optional email and password accounts for cloud projects.
				</p>
				<p>
					Account email and display name are stored to provide sign-in and
					associate cloud projects with their owner. Passwords are handled by the
					authentication service and are not stored in plain text.
				</p>
				<p>
					Projects you do not sync remain in your browser. Synced project data is
					stored in PostgreSQL and media files are kept in persistent server
					storage, associated with your account.
				</p>
			</section>

			<section className="flex flex-col gap-3">
				<h2 className="text-2xl font-semibold">Analytics</h2>
				<p>
					We use{" "}
					<a
						href="https://www.databuddy.cc"
						target="_blank"
						rel="noopener noreferrer"
						className="text-primary hover:underline"
					>
						Databuddy
					</a>{" "}
					for basic, anonymized visitor counts. We do not track clicks,
					interactions, or how you use the editor.
				</p>
				<p>
					No personal information is collected, no individual users are tracked,
					and no data that could identify you is stored.
				</p>
			</section>

			<section className="flex flex-col gap-3">
				<h2 className="text-2xl font-semibold">Local Storage & Cookies</h2>
				<p>We use browser local storage and IndexedDB to:</p>
				<ul className="list-disc space-y-2 pl-6">
					<li>Cache project data and media locally for editing</li>
					<li>Remember your editor preferences and settings</li>
					<li>Store app state needed for the editor to work between sessions</li>
				</ul>
				<p>
					Local cached data can be cleared in browser settings. Synced projects
					can be deleted from your account by deleting the project.
				</p>
			</section>

			<section className="flex flex-col gap-3">
				<h2 className="text-2xl font-semibold">Third-Party Services</h2>
				<p>OpenCut integrates with these services:</p>
				<ul className="list-disc space-y-2 pl-6">
					<li>
						<strong>Vercel:</strong> For hosting and content delivery
					</li>
					<li>
						<strong>Databuddy:</strong> For anonymized analytics
					</li>
				</ul>
			</section>

			<section className="flex flex-col gap-3">
				<h2 className="text-2xl font-semibold">Your Rights</h2>
				<p>You have complete control over your data:</p>
				<ul className="list-disc space-y-2 pl-6">
					<li>No account is required to use OpenCut today</li>
					<li>Clear local storage to remove all saved projects</li>
					<li>Contact us with any privacy concerns</li>
				</ul>
			</section>

			<section className="flex flex-col gap-3">
				<h2 className="text-2xl font-semibold">Open Source Transparency</h2>
				<p>
					OpenCut is completely open source. You can review our code, see
					exactly how we handle data, and even self-host the application if you
					prefer.
				</p>
				<p>
					View our source code on{" "}
					<a
						href={SOCIAL_LINKS.github}
						target="_blank"
						rel="noopener noreferrer"
						className="text-primary hover:underline"
					>
						GitHub
					</a>
					.
				</p>
			</section>

			<section className="flex flex-col gap-3">
				<h2 className="text-2xl font-semibold">Contact Us</h2>
				<p>Questions about this privacy policy or how we handle your data?</p>
				<p>
					Open an issue on our{" "}
					<a
						href={`${SOCIAL_LINKS.github}/issues`}
						target="_blank"
						rel="noopener noreferrer"
						className="text-primary hover:underline"
					>
						GitHub repository
					</a>
					, email us at{" "}
					<a
						href="mailto:oss@opencut.app"
						className="text-primary hover:underline"
					>
						oss@opencut.app
					</a>
					, or reach out on{" "}
					<a
						href={SOCIAL_LINKS.x}
						target="_blank"
						rel="noopener noreferrer"
						className="text-primary hover:underline"
					>
						X (Twitter)
					</a>
					.
				</p>
			</section>

			<Separator />

			<p className="text-muted-foreground text-sm">
				Last updated: March 15, 2026
			</p>
		</BasePage>
	);
}
